#!/usr/bin/env python3

import os
import argparse
import shutil
import requests
import json

ap = argparse.ArgumentParser()

ap.add_argument('--force', action='store_true')

a = ap.parse_args()

from neotermcolor import colored

separator = colored('-', color='grey') * 40

version = open('VERSION').read().strip()

work_dir = os.getcwd()


def sh(cmd):
    if os.system(cmd):
        raise RuntimeError


targets = [{
    'sfx': 'aarch64-musl',
    'target': 'aarch64-unknown-linux-musl'
}, {
    'sfx': 'x86_64-musl',
    'target': 'x86_64-unknown-linux-musl'
}]

binaries = [
    'sim-modbus-generic', 'sim-modbus-port', 'sim-modbus-relay',
    'sim-modbus-sensor'
]

tarballs = []

if not a.force:
    r = requests.head(
        f'https://pub.bma.ai/sim/{version}/sim-{version}-{targets[0]["sfx"]}.tgz'
    )
    if r.ok:
        raise RuntimeError('version already released')
    elif r.status_code != 404:
        raise RuntimeError('unable to check released version')

try:
    shutil.rmtree('_build')
except FileNotFoundError:
    pass

for t in targets:
    print('Building', colored(t['target'], color='blue'))
    sfx = t['sfx']
    sh(f'cross build --release --target {t["target"]}')
    name = f'sim-{version}-{sfx}'
    tarball = f'{name}.tgz'
    d = f'_build/{name}'
    os.makedirs(d)
    os.chdir(d)
    for f in binaries:
        sh(f'cp -vf {work_dir}/target/{t["target"]}/release/{f} .')
    sh(f'cp -vf {work_dir}/svc-tpl/* .')
    os.chdir('..')
    sh(f'tar czvf {tarball} {name}')
    tarballs.append(tarball)
    os.chdir(work_dir)
    print(separator)

print(colored('Uploading...', color='cyan'))
for f in tarballs:
    sh(f'gsutil cp -a public-read _build/{f} gs://pub.bma.ai/sim/{version}/')

print(colored('Releasing...', color='cyan'))
f_update_info = '_build/update_info.json'
json.dump({'version': version}, open(f_update_info, 'w'))
sh(f'gsutil -h "Cache-Control:no-cache" cp -a public-read {f_update_info} gs://pub.bma.ai/sim/'
  )
