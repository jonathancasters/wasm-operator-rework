# Ring Operators Example

This example demonstrates a "ring" of operators, where two operators reconcile the `TestResource` custom resources
between two namespaces. It's meant to be used up
to [this commit](https://github.com/jonathancasters/wasm-operator-rework/commit/328bea268d9645aefaf6a74064d4aa08d08c7770).

## Overview

This example consists of two operators, one written in Go and the other in Rust. They are configured to watch for
`TestResource` resources in two different namespaces, `ring-a` and `ring-b`.

- The **Go operator** watches for resources in `ring-a` and reconciles them to `ring-b`.
- The **Rust operator** watches for resources in `ring-b` and reconciles them to `ring-a`.

This creates a "ring" where a change in one namespace will be propagated to the other.

## How it Works

The core logic of both operators is the same:

1. List all `TestResource` objects in the input namespace.
2. For each resource, check if a corresponding resource exists in the output namespace.
3. If the output resource does not exist, create it.
4. If the output resource exists, compare the `nonce` value in the `spec` of the input resource with the `nonce` of the
   output resource.
5. If the input `nonce` is greater than the output `nonce`, update the output resource with the new `nonce`.

This `nonce` check prevents an infinite loop of updates between the two operators.

## Custom Resource

The `TestResource` custom resource is defined in `crd.yaml`. It has a simple schema with a `spec.nonce` field of type
`integer`.

## Configuration

The `configuration.yaml` file defines the two operators and their environment variables:

- `IN_NAMESPACE`: The namespace the operator watches for resources.
- `OUT_NAMESPACE`: The namespace the operator reconciles resources to.

## Running the Example

To run the example, you can use the `run.sh` script. This will apply the `crd.yaml`, create the two namespaces, and run
the operator with the specified configuration.