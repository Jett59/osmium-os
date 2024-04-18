#!/bin/sh

grep -iv '\.ltmp' $1 | grep -iv '\.lfunc' | grep -iv '\.file' | grep -iv '\.type' | grep -iv '\.section' | grep -iv '\.cfi' | grep -iv '\.loc'
