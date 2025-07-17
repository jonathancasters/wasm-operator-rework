#!/bin/bash

go get -tool go.bytecodealliance.org/cmd/wit-bindgen-go
go tool wit-bindgen-go generate --world component-world --out internal ./wit