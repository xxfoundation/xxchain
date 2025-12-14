#!/bin/bash
# Wrapper for chopsticks that filters out the @polkadot version warnings
# These warnings are a known upstream issue and don't affect functionality

chopsticks "$@" 2>&1 | grep -v "^@polkadot" | grep -v "multiple versions" | grep -v "conflicting packages" | grep -v "^	cjs" | grep -v "Either remove"
