#!/usr/bin/env python3

import os
import toml

version = open("VERSION").read().strip()

for member in toml.load(open('Cargo.toml'))['workspace']['members']:
    if os.system(
            f"""sed -i 's/^version = .*/version = "{version}"/g' {member}/Cargo.toml"""
    ):
        raise RuntimeError
