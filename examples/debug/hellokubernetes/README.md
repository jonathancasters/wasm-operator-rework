# Kubernetes requests

In this example we want to show that we can make a request from a wasm module to the kubernetes cluster through the
parent

Therefor we need:

- A 'kind' cluster to which the parent can connect
- A child wasm module which will use the wit interface of the parent to make the request (kubernetes-requests)

We will make sure there is some dummy pod and we can list the pods in the cluster from the child