#!/usr/bin/env python
import json
import re
import sys

from pygit2 import Repository

repo = Repository(".git")
commit2message = {
    commit.id.hex: commit.message for commit in repo.walk(repo.branches.get("master").target)
}

input = json.load(sys.stdin)

output = []
for row in input["results"]:
    dir, num, commit = re.match("([^/]+/)?([0-9]+)-([0-9a-f]+) .*", row["command"]).groups()
    for time in row["times"]:
        output_row = row.copy()
        output_row["commit"] = f"{num}-{commit}"
        output_row["message"] = commit2message.get(commit, "")
        del output_row["times"]
        output_row["time"] = time

        output.append(output_row)

json.dump(output, sys.stdout, indent=2)
