#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd -P)
checker="$root/scripts/external-evidence-register-check.sh"
header=$(cat "$root/docs/EXTERNAL_EVIDENCE_REGISTER.csv")
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
register="$work/register.csv"
attestation="$work/attestation.conf"
source_repository="$work/source"
git init -q "$source_repository"
git -C "$source_repository" -c user.name='CQ test' -c user.email='cq-test.invalid' commit --allow-empty -q -m fixture
git -C "$source_repository" -c user.name='CQ test' -c user.email='cq-test.invalid' tag -a v0.20.0 -m fixture
target_commit=$(git -C "$source_repository" rev-parse HEAD)
printf '%s\n' "$header" > "$register"

digest() { printf 'sha256:%064x' "$1"; }
row() {
  local id=$1 type=$2 organisation=$3 project=$4 domain=$5 worker=$6 expected=$7 index=$8
  local exit_class=0 replay=na triaged=na bundle isolation
  case "$expected" in
    SAFE) exit_class=0 ;;
    UNSAFE) exit_class=1; replay=yes; triaged=yes ;;
    FAILURE) exit_class=2 ;;
    *) return 2 ;;
  esac
  if [[ $expected == FAILURE ]]; then
    bundle= isolation=
  else
    bundle=$(digest $((1000 + index)))
    isolation=$(digest $((2000 + index)))
  fi
  printf '%s\n' "$id,$type,custodian-$type,$organisation,$project,$domain,$worker,v0.20.0,$target_commit,env-$worker,$(digest "$index"),$(digest $((500 + index))),oracle-$index,$expected,$expected,$expected,$exit_class,$bundle,$isolation,$((index + 1)),$((4096 + index)),$replay,$triaged,yes,yes,accepted,reviewed,report-$id" >> "$register"
}

for index in {1..7}; do row "security-$index" security-review assessor-org security-target security worker-sec FAILURE "$index"; done
row technical-1 technical-review reviewer-org technical-target formal worker-tech SAFE 101
row technical-2 technical-review reviewer-org technical-target formal worker-tech UNSAFE 102
row technical-3 technical-review reviewer-org technical-target formal worker-tech FAILURE 103
for index in {1..30}; do
  if (( index <= 5 )); then expected=SAFE
  elif (( index <= 10 )); then expected=UNSAFE
  elif (( index <= 15 )); then expected=FAILURE
  else expected=SAFE
  fi
  if (( index <= 15 )); then organisation=partner-a; worker=worker-a; domain=medical
  else organisation=partner-b; worker=worker-b; domain=industrial
  fi
  if (( index <= 10 )); then project=project-a
  elif (( index <= 20 )); then project=project-b
  else project=project-c
  fi
  row "partner-$index" partner-pilot "$organisation" "$project" "$domain" "$worker" "$expected" $((200 + index))
done

cat > "$attestation" <<EOF
protocol_version=1
target_tag=v0.20.0
target_commit=$target_commit
security_assessment_status=PASS
security_assessment_report=security-report-v1
technical_review_status=PASS
technical_review_report=technical-report-v1
operator_exercises_status=PASS
data_handling_status=PASS
independent_reviewer_id=reviewer-1
independent_aggregate_status=PASS
independent_aggregate_report=aggregate-report-v1
critical_findings_open=0
high_findings_open=0
assessment_date=2026-07-17
EOF

"$checker" "$root/docs/EXTERNAL_EVIDENCE_REGISTER.csv" | grep -qx 'external-evidence-register status=VALID rows=0'
"$checker" "$register" | grep -qx 'external-evidence-register status=VALID rows=40'
"$checker" --production-gate "$register" "$attestation" "$source_repository" | grep -qx 'external-production-gate status=PASS rows=40'

cp "$register" "$work/bad-disagreement.csv"
sed -i.bak 's/,SAFE,SAFE,SAFE,0,/,SAFE,UNSAFE,SAFE,0,/' "$work/bad-disagreement.csv"
if "$checker" --production-gate "$work/bad-disagreement.csv" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted a result disagreement" >&2
  exit 1
fi

cp "$register" "$work/bad-exit.csv"
sed -i.bak 's/,UNSAFE,UNSAFE,UNSAFE,1,/,UNSAFE,UNSAFE,UNSAFE,0,/' "$work/bad-exit.csv"
if "$checker" --production-gate "$work/bad-exit.csv" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted a result/exit mismatch" >&2
  exit 1
fi

cp "$register" "$work/bad-witness.csv"
sed -i.bak 's/,yes,yes,yes,yes,accepted,reviewed,report-partner-6$/,no,yes,yes,yes,accepted,reviewed,report-partner-6/' "$work/bad-witness.csv"
if "$checker" --production-gate "$work/bad-witness.csv" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted an unreplayed UNSAFE witness" >&2
  exit 1
fi

cp "$register" "$work/bad-open.csv"
sed -i.bak '2s/,accepted,reviewed,/,open,reviewed,/' "$work/bad-open.csv"
if "$checker" --production-gate "$work/bad-open.csv" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted an open disposition" >&2
  exit 1
fi

cp "$register" "$work/bad-formula.csv"
sed -i.bak '2s/^security-1/=security-1/' "$work/bad-formula.csv"
if "$checker" "$work/bad-formula.csv" >/dev/null 2>&1; then
  echo "validator accepted a spreadsheet formula" >&2
  exit 1
fi

cp "$attestation" "$work/bad-attestation.conf"
sed -i.bak 's/security_assessment_status=PASS/security_assessment_status=FAIL/' "$work/bad-attestation.conf"
if "$checker" --production-gate "$register" "$work/bad-attestation.conf" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted a failed security assessment" >&2
  exit 1
fi

cp "$attestation" "$work/bad-date.conf"
sed -i.bak 's/assessment_date=2026-07-17/assessment_date=2026-99-99/' "$work/bad-date.conf"
if "$checker" --production-gate "$register" "$work/bad-date.conf" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted an invalid assessment date" >&2
  exit 1
fi

head -n 30 "$register" > "$work/undersized.csv"
if "$checker" --production-gate "$work/undersized.csv" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted an undersized cohort" >&2
  exit 1
fi

git -C "$source_repository" tag -d v0.20.0 >/dev/null
git -C "$source_repository" tag v0.20.0
if "$checker" --production-gate "$register" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted a lightweight target tag" >&2
  exit 1
fi

git -C "$source_repository" tag -d v0.20.0 >/dev/null
git -C "$source_repository" -c user.name='CQ test' -c user.email='cq-test.invalid' commit --allow-empty -q -m other
git -C "$source_repository" -c user.name='CQ test' -c user.email='cq-test.invalid' tag -a v0.20.0 -m other
if "$checker" --production-gate "$register" "$attestation" "$source_repository" >/dev/null 2>&1; then
  echo "production gate accepted a tag/commit mismatch" >&2
  exit 1
fi

ln -s "$register" "$work/register-link.csv"
if "$checker" "$work/register-link.csv" >/dev/null 2>&1; then
  echo "validator accepted a symlink register" >&2
  exit 1
fi

echo 'external-evidence-register-tests=PASS'
