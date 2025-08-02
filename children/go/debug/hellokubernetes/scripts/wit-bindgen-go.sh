#!/bin/bash

set -e

# Make sure we have the wit dependencies
echo "Fetching dependencies WIT..."
wkg wit fetch
echo "Generating Go bindings for WIT..."
rm -rdf internal
go generate
echo "Finished generating Go bindings"