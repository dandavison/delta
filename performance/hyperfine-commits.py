#!/usr/bin/env python
import argparse
import os
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("input_file", help="Input file for each command")
parser.add_argument("executables_dir", help="Directory of executables to be benchmarked")
parser.add_argument("output_file", help="File to receive JSON output")
args = parser.parse_args()

runs = 5
hyperfine_args = [
    "hyperfine",
    "--runs",
    f"{runs}",
    "--export-json",
    f"{args.output_file}",
    "--ignore-failure",
]
hyperfine_args.extend(
    f"{args.executables_dir}/{executable} < {args.input_file} > /dev/null"
    for executable in sorted(os.listdir(args.executables_dir))
)
subprocess.check_call(hyperfine_args)
