#!/bin/bash

set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <number_of_operators>"
  exit 1
fi

NUM_OPERATORS=$1
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Generate Namespaces first
for i in $(seq 1 $NUM_OPERATORS); do
  cat <<EOF
apiVersion: v1
kind: Namespace
metadata:
  name: operator-$i
---
EOF
done

# Generate CRD
cat <<EOF
apiVersion: "apiextensions.k8s.io/v1"
kind: "CustomResourceDefinition"
metadata:
  name: "rings.example.com"
spec:
  group: "example.com"
  names:
    kind: "Ring"
    plural: "rings"
    singular: "ring"
    shortNames:
    - "rg"
  scope: "Namespaced"
  versions:
  - name: "v1"
    served: true
    storage: true
    schema:
      openAPIV3Schema:
        type: "object"
        properties:
          spec:
            type: "object"
            properties:
              replicas:
                type: "integer"
              nonce:
                type: "string"
              chain:
                type: "array"
                items:
                  type: "string"
              hop:
                type: "integer"
---
EOF

# Generate RBAC
cat <<EOF
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: ring-operator-bare-metal-role
rules:
- apiGroups: ["example.com"]
  resources: ["rings"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list", "watch", "create", "delete"]
- apiGroups: [""]
  resources: ["namespaces"]
  verbs: ["create"]
---
EOF

for i in $(seq 1 $NUM_OPERATORS); do
  cat <<EOF
apiVersion: v1
kind: ServiceAccount
metadata:
  name: ring-operator-bare-metal-sa
  namespace: operator-$i
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: ring-operator-bare-metal-rb-$i
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: ring-operator-bare-metal-role
subjects:
- kind: ServiceAccount
  name: ring-operator-bare-metal-sa
  namespace: operator-$i
---
EOF
done

# Generate Deployments
for i in $(seq 1 $NUM_OPERATORS); do
  sed "s/{{OPERATOR_INDEX}}/$i/g" "$SCRIPT_DIR/k8s/deployment.yaml.template"
  echo
  echo "---"
done