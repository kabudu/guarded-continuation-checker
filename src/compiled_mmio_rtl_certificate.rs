//! Canonical proof-carrying evidence for the retained compiled-firmware to
//! OpenTitan PWM RTL composition.

use crate::{
    compiled_mmio_predicate_certificate::{
        CompiledMmioPredicateCertificateVerification, certify_compiled_mmio_predicate,
        decode_compiled_mmio_predicate_certificate, encode_compiled_mmio_predicate_certificate,
        verify_compiled_mmio_predicate_bytes,
    },
    compiled_mmio_rtl_mapping::{
        PWM_RTL_CHANNELS, PWM_RTL_EXTENDED_TRACE_FRAMES, PWM_RTL_MAPPING_VERSION,
        PWM_RTL_MODEL_SHA256, PwmRtlObservation, PwmRtlReplay,
        extend_pwm_rtl_trace_one_phase_cycle, map_pwm_mmio_workflow, replay_pwm_rtl_trace,
    },
    riscv32imc::Rv32SymbolLayout,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

const MAGIC: &[u8; 8] = b"GCCMRTL1";
pub const COMPILED_MMIO_RTL_CERTIFICATE_VERSION: u32 = 1;
pub const MAX_COMPILED_MMIO_RTL_CERTIFICATE_BYTES: usize = 8 * 1024 * 1024;
const CHECKSUM_BYTES: usize = 32;
const OBSERVATIONS_PER_MEMBER: usize = PWM_RTL_EXTENDED_TRACE_FRAMES + 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioRtlCertificate {
    pub version: u32,
    pub predicate_certificate: Vec<u8>,
    pub rtl_model_sha256: [u8; 32],
    pub members: Vec<PwmRtlReplay>,
    pub invalid_rtl_members: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMmioRtlCertificateVerification {
    pub artifact_bytes: u32,
    pub predicate: CompiledMmioPredicateCertificateVerification,
    pub valid_rtl_members: u32,
    pub invalid_rtl_members: u32,
    pub rtl_transitions: u32,
    pub rtl_observations: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMmioRtlCertificateError(pub String);

impl fmt::Display for CompiledMmioRtlCertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "compiled-MMIO RTL certificate: {}", self.0)
    }
}

impl Error for CompiledMmioRtlCertificateError {}

fn reject(message: impl Into<String>) -> CompiledMmioRtlCertificateError {
    CompiledMmioRtlCertificateError(message.into())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn validate(
    certificate: &CompiledMmioRtlCertificate,
) -> Result<(), CompiledMmioRtlCertificateError> {
    if certificate.version != COMPILED_MMIO_RTL_CERTIFICATE_VERSION {
        return Err(reject("unsupported certificate version"));
    }
    decode_compiled_mmio_predicate_certificate(&certificate.predicate_certificate)
        .map_err(|error| reject(error.to_string()))?;
    if certificate.rtl_model_sha256 != PWM_RTL_MODEL_SHA256 {
        return Err(reject("RTL model identity is not canonical"));
    }
    if certificate.invalid_rtl_members != 0 || certificate.members.len() != PWM_RTL_CHANNELS {
        return Err(reject("RTL member counts are not canonical"));
    }
    for (channel, member) in certificate.members.iter().enumerate() {
        if member.version != PWM_RTL_MAPPING_VERSION
            || member.channel as usize != channel
            || member.transitions != PWM_RTL_EXTENDED_TRACE_FRAMES as u32
            || member.observations.len() != OBSERVATIONS_PER_MEMBER
            || member
                .observations
                .iter()
                .any(|observation| observation.step > 0x0f || observation.pwm > 0x3f)
        {
            return Err(reject(format!("RTL member {channel} is not canonical")));
        }
    }
    Ok(())
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub fn encode_compiled_mmio_rtl_certificate(
    certificate: &CompiledMmioRtlCertificate,
) -> Result<Vec<u8>, CompiledMmioRtlCertificateError> {
    validate(certificate)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_u32(&mut bytes, certificate.version);
    push_u32(
        &mut bytes,
        u32::try_from(certificate.predicate_certificate.len())
            .map_err(|_| reject("predicate certificate length exceeds u32"))?,
    );
    bytes.extend_from_slice(&certificate.predicate_certificate);
    bytes.extend_from_slice(&certificate.rtl_model_sha256);
    push_u32(
        &mut bytes,
        u32::try_from(certificate.members.len())
            .map_err(|_| reject("RTL member count exceeds u32"))?,
    );
    for member in &certificate.members {
        bytes.push(member.channel);
        push_u32(&mut bytes, member.transitions);
        push_u32(
            &mut bytes,
            u32::try_from(member.observations.len())
                .map_err(|_| reject("RTL observation count exceeds u32"))?,
        );
        for observation in &member.observations {
            bytes.push(observation.step);
            bytes.push(observation.pwm);
        }
    }
    push_u32(&mut bytes, certificate.invalid_rtl_members);
    bytes.extend_from_slice(&digest(&bytes));
    if bytes.len() > MAX_COMPILED_MMIO_RTL_CERTIFICATE_BYTES {
        return Err(reject("encoded certificate exceeds policy"));
    }
    Ok(bytes)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], CompiledMmioRtlCertificateError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| reject("certificate offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| reject("certificate is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, CompiledMmioRtlCertificateError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, CompiledMmioRtlCertificateError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| reject("invalid u32 field"))?,
        ))
    }
}

pub fn decode_compiled_mmio_rtl_certificate(
    bytes: &[u8],
) -> Result<CompiledMmioRtlCertificate, CompiledMmioRtlCertificateError> {
    if bytes.len() < 128 || bytes.len() > MAX_COMPILED_MMIO_RTL_CERTIFICATE_BYTES {
        return Err(reject("certificate size is outside policy"));
    }
    let content_len = bytes
        .len()
        .checked_sub(CHECKSUM_BYTES)
        .ok_or_else(|| reject("certificate is truncated"))?;
    if digest(&bytes[..content_len]) != bytes[content_len..] {
        return Err(reject("certificate checksum mismatch"));
    }
    let mut cursor = Cursor {
        bytes: &bytes[..content_len],
        offset: 0,
    };
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(reject("certificate magic mismatch"));
    }
    let version = cursor.u32()?;
    let predicate_len = usize::try_from(cursor.u32()?)
        .map_err(|_| reject("predicate certificate length exceeds platform range"))?;
    let predicate_certificate = cursor.take(predicate_len)?.to_vec();
    let rtl_model_sha256 = cursor
        .take(32)?
        .try_into()
        .map_err(|_| reject("invalid RTL model digest"))?;
    let member_count = usize::try_from(cursor.u32()?)
        .map_err(|_| reject("RTL member count exceeds platform range"))?;
    if member_count != PWM_RTL_CHANNELS {
        return Err(reject("RTL member count is not canonical"));
    }
    let mut members = Vec::with_capacity(member_count);
    for _ in 0..member_count {
        let channel = cursor.u8()?;
        let transitions = cursor.u32()?;
        let observation_count = usize::try_from(cursor.u32()?)
            .map_err(|_| reject("RTL observation count exceeds platform range"))?;
        if observation_count != OBSERVATIONS_PER_MEMBER {
            return Err(reject("RTL observation count is not canonical"));
        }
        let mut observations = Vec::with_capacity(observation_count);
        for _ in 0..observation_count {
            observations.push(PwmRtlObservation {
                step: cursor.u8()?,
                pwm: cursor.u8()?,
            });
        }
        members.push(PwmRtlReplay {
            version: PWM_RTL_MAPPING_VERSION,
            channel,
            observations,
            transitions,
        });
    }
    let invalid_rtl_members = cursor.u32()?;
    if cursor.offset != cursor.bytes.len() {
        return Err(reject("certificate has trailing content"));
    }
    let certificate = CompiledMmioRtlCertificate {
        version,
        predicate_certificate,
        rtl_model_sha256,
        members,
        invalid_rtl_members,
    };
    validate(&certificate)?;
    if encode_compiled_mmio_rtl_certificate(&certificate)? != bytes {
        return Err(reject("certificate encoding is not canonical"));
    }
    Ok(certificate)
}

pub fn produce_compiled_mmio_rtl_certificate(
    image: &[u8],
    symbols: Rv32SymbolLayout,
    model_bytes: &[u8],
) -> Result<Vec<u8>, CompiledMmioRtlCertificateError> {
    if digest(model_bytes) != PWM_RTL_MODEL_SHA256 {
        return Err(reject(
            "RTL model identity differs from the pinned source boundary",
        ));
    }
    let predicate = certify_compiled_mmio_predicate(image, symbols)
        .map_err(|error| reject(error.to_string()))?;
    let predicate_certificate = encode_compiled_mmio_predicate_certificate(&predicate)
        .map_err(|error| reject(error.to_string()))?;
    let family =
        map_pwm_mmio_workflow(&predicate.workflow).map_err(|error| reject(error.to_string()))?;
    let members = family
        .traces
        .iter()
        .map(|trace| {
            let extended = extend_pwm_rtl_trace_one_phase_cycle(trace)
                .map_err(|error| reject(error.to_string()))?;
            replay_pwm_rtl_trace(model_bytes, &extended).map_err(|error| reject(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    encode_compiled_mmio_rtl_certificate(&CompiledMmioRtlCertificate {
        version: COMPILED_MMIO_RTL_CERTIFICATE_VERSION,
        predicate_certificate,
        rtl_model_sha256: PWM_RTL_MODEL_SHA256,
        members,
        invalid_rtl_members: family.invalid_rtl_members,
    })
}

pub fn verify_compiled_mmio_rtl_certificate(
    bytes: &[u8],
    image: &[u8],
    symbols: Rv32SymbolLayout,
    model_bytes: &[u8],
) -> Result<CompiledMmioRtlCertificateVerification, CompiledMmioRtlCertificateError> {
    let certificate = decode_compiled_mmio_rtl_certificate(bytes)?;
    if digest(model_bytes) != certificate.rtl_model_sha256 {
        return Err(reject("RTL model source identity mismatch"));
    }
    let predicate =
        verify_compiled_mmio_predicate_bytes(&certificate.predicate_certificate, image, symbols)
            .map_err(|error| reject(error.to_string()))?;
    let decoded_predicate =
        decode_compiled_mmio_predicate_certificate(&certificate.predicate_certificate)
            .map_err(|error| reject(error.to_string()))?;
    let family = map_pwm_mmio_workflow(&decoded_predicate.workflow)
        .map_err(|error| reject(error.to_string()))?;
    if family.invalid_rtl_members != certificate.invalid_rtl_members {
        return Err(reject(
            "invalid RTL member count differs after reconstruction",
        ));
    }
    let expected = family
        .traces
        .iter()
        .map(|trace| {
            let extended = extend_pwm_rtl_trace_one_phase_cycle(trace)
                .map_err(|error| reject(error.to_string()))?;
            replay_pwm_rtl_trace(model_bytes, &extended).map_err(|error| reject(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if expected != certificate.members {
        return Err(reject(
            "RTL replay observations differ after reconstruction",
        ));
    }
    let valid_rtl_members =
        u32::try_from(expected.len()).map_err(|_| reject("valid RTL member count exceeds u32"))?;
    let rtl_transitions = expected.iter().try_fold(0u32, |total, member| {
        total
            .checked_add(member.transitions)
            .ok_or_else(|| reject("RTL transition count overflow"))
    })?;
    let rtl_observations = expected.iter().try_fold(0u32, |total, member| {
        total
            .checked_add(
                u32::try_from(member.observations.len())
                    .map_err(|_| reject("RTL observation count exceeds u32"))?,
            )
            .ok_or_else(|| reject("RTL observation count overflow"))
    })?;
    Ok(CompiledMmioRtlCertificateVerification {
        artifact_bytes: u32::try_from(bytes.len())
            .map_err(|_| reject("artifact byte count exceeds u32"))?,
        predicate,
        valid_rtl_members,
        invalid_rtl_members: certificate.invalid_rtl_members,
        rtl_transitions,
        rtl_observations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_truncated_hostile_and_oversized_inputs_before_allocation() {
        assert!(decode_compiled_mmio_rtl_certificate(&[]).is_err());
        assert!(decode_compiled_mmio_rtl_certificate(&[0; 127]).is_err());
        assert!(
            decode_compiled_mmio_rtl_certificate(&vec![
                0;
                MAX_COMPILED_MMIO_RTL_CERTIFICATE_BYTES + 1
            ])
            .is_err()
        );
    }
}
