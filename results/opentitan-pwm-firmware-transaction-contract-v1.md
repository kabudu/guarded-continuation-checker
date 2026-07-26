# OpenTitan PWM firmware transaction-contract result v1

The bounded cross-layer mechanism passes. It is not a novelty claim.

One canonical 128,892-byte envelope binds:

- the authored four-event interpretation of OpenTitan's published PWM
  programming sequence;
- its exact mapping to the retained crosstalk fixture;
- a trace that reaches the observation-ready state without rejection; and
- the existing two-atom revision-impact evidence.

Independent verification preserves all 20 old/new RTL observations and the
three minimal semantic-change sets. In particular, the valid firmware contract
does not hide the authentic defect:

| Query | Old core | New core |
| --- | --- | --- |
| Core-only crosstalk | UNSAFE | SAFE |
| Joint crosstalk with both channel revisions | UNSAFE | SAFE |

Four invalid schedules refuse before any RTL envelope is produced:

- enabling channel 0 before configuration;
- omitting the final observation;
- disabling channel 0 before observation; and
- reconfiguring the active channel instead of the unrelated channel.

The exact envelope is 124 bytes larger than the existing 128,768-byte
revision-impact bundle. It adds a source-bound firmware precondition and
fail-closed schedule boundary, but it does not reduce proof work or evidence
size. The previously qualified maintained Yosys, rIC3 and Certifaiger route
still uses only 15,479 model-plus-evidence bytes for the RTL matrix.

The contract text and stimulus mapping are GCC-authored interpretations of the
public programmer's guide, not extracted production firmware. This closes a
mechanism gate only. Authentic compiled-firmware extraction and an
equivalent-scope contract-aware baseline remain open.
