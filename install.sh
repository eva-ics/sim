#!/bin/sh

REPO=https://pub.bma.ai/sim
BINARIES='sim-modbus-generic sim-modbus-port sim-modbus-relay sim-modbus-sensor'

FULL_NAME='EVA ICS Virtual Fieldbus Simulator'

[ -z "$EVA_DIR" ] && EVA_DIR=/opt/eva4
[ -z "$TARGET_DIR" ] && TARGET_DIR="${EVA_DIR}/sim"

case $(uname -m) in
  x86_64)
    ARCH_SFX=x86_64-musl
    ;;
  aarch64)
    ARCH_SFX=aarch64-musl
    ;;
  *)
    echo "Unsupported CPU architecture"
    exit 8
    ;;
esac

UPDATE_FILE=/tmp/sim_update_info.jsom

if ! curl -Ls "${REPO}/update_info.json" -o "${UPDATE_FILE}"; then
  echo "Unable to download ${FULL_NAME} update info"
  exit 7
fi

VERSION=$(jq -r .version "${UPDATE_FILE}")

rm -f "${UPDATE_FILE}"

if [ -z "$VERSION" ]; then
  echo "Unable to obtain version info"
  exit 8
fi

mkdir -p "${TARGET_DIR}"

DISTRO="${REPO}/${VERSION}/sim-${VERSION}-${ARCH_SFX}.tgz"

TMP_DIR=/tmp/eva_sim

rm -rf "$TMP_DIR"
mkdir "$TMP_DIR" || exit 1

echo "Downloading ${DISTRO}"
curl -L "${DISTRO}" | \
  tar xzf - --strip-components=1 -C "${TMP_DIR}/" || exit 1

echo "Installing..."
for f in BINARIES; do
  rm -f "${TARGET_DIR}/${f}" || exit 1
done
cp -rvf ${TMP_DIR}/* "${TARGET_DIR}/" || exit 2

echo "Cleaning up...."
rm -rf "${TMP_DIR}"

echo
echo "${FULL_NAME} has been successfully installed/updated"
echo "version: ${VERSION}"
echo
