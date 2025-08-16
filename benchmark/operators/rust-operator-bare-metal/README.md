# Bare-Metal Rust Ring Operator

This directory contains a bare-metal Rust implementation of the ring operator for the benchmark tests. This operator is built using the `kube-rs` library and interacts directly with the Kubernetes API, without using the Wasm-based parent operator framework.

## Functionality

The operator watches for `Ring` custom resources. When a `Ring` resource is created or updated, the operator reconciles the state of the cluster to match the `spec.replicas` field of the `Ring` resource. It does this by creating or deleting pods to match the desired number of replicas.

The operator also updates the `status.reconciledReplicas` field of the `Ring` resource to reflect the current state.

## How to Build

To build the operator and the Docker image, run the following command:

```bash
./compile.sh
```

This will create a Docker image named `ring-operator-bare-metal:latest`.

## How to Deploy

Before deploying, you need to push the Docker image to a container registry that your Kubernetes cluster can access. Then, update the `k8s/deployment.yaml` file with the correct image name.

Once the image is available to your cluster, you can deploy the operator by running:

```bash
kubectl apply -f k8s/
```

This will create the `Ring` CustomResourceDefinition, the necessary RBAC roles, and the deployment for the operator.

## How to Run the Benchmark

To run the benchmark for this operator, use the `run_all_benchmarks_bare_metal.sh` script located in the `benchmark` directory:

```bash
../../run_all_benchmarks_bare_metal.sh
```

This script will run the benchmark with different numbers of operators and record the latency and memory usage.
