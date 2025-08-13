//go:generate go run go.bytecodealliance.org/cmd/wit-bindgen-go generate --world child-world --out internal/ ../../../parent/wit
package main

import (
	"encoding/json"
	"go.bytecodealliance.org/cm"
	"ring-operator-go/internal/local/operator/kube-operator"
	"ring-operator-go/internal/local/operator/kubernetes"
	"ring-operator-go/internal/local/operator/types"
	"ring-operator-go/internal/wasi/cli/environment"
)

// RingResource Structs for parsing the Ring resource JSON
type RingResource struct {
	ApiVersion string     `json:"apiVersion"`
	Kind       string     `json:"kind"`
	Metadata   ObjectMeta `json:"metadata"`
	Spec       RingSpec   `json:"spec"`
}

type ObjectMeta struct {
	Name      string `json:"name"`
	Namespace string `json:"namespace"`
}

type RingSpec struct {
	TargetNamespace string `json:"targetNamespace"`
}

func init() {
	kubeoperator.Exports.GetWatchRequests = GetWatchRequests
	kubeoperator.Exports.Serialize = Serialize
	kubeoperator.Exports.Deserialize = Deserialize
	kubeoperator.Exports.Reconcile = Reconcile
}

func GetWatchRequests() cm.List[types.WatchRequest] {
	ns := ""
	for _, pair := range environment.GetEnvironment().Slice() {
		if pair[0] == "IN_NAMESPACE" {
			ns = pair[1]
			break
		}
	}

	if ns == "" {
		kubernetes.Log(types.LogLevelError, "IN_NAMESPACE environment variable not set")
		return cm.List[types.WatchRequest]{}
	}

	return cm.ToList([]types.WatchRequest{
		{Kind: "Ring", Namespace: ns},
	})
}

func Reconcile(req kubeoperator.ReconcileRequest) kubeoperator.ReconcileResult {
	// Log the start of the reconciliation
	kubernetes.Log(types.LogLevelInfo, "Go operator reconciling...")

	// 1. Parse the incoming resource
	var originalRing RingResource
	err := json.Unmarshal([]byte(req.ResourceJSON), &originalRing)
	if err != nil {
		msg := "Error parsing resource JSON: " + err.Error()
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	logMsg := "Original ring: " + originalRing.Metadata.Name + " in namespace " + originalRing.Metadata.Namespace
	kubernetes.Log(types.LogLevelInfo, logMsg)

	// 2. Construct the new Ring resource
	newRing := RingResource{
		ApiVersion: "example.com/v1",
		Kind:       "Ring",
		Metadata: ObjectMeta{
			Name:      originalRing.Metadata.Name,
			Namespace: originalRing.Spec.TargetNamespace, // Target namespace becomes the new namespace
		},
		Spec: RingSpec{
			TargetNamespace: originalRing.Metadata.Namespace, // The original namespace is the next target
		},
	}

	// 3. Marshal the new ring to JSON
	newRingJson, err := json.Marshal(newRing)
	if err != nil {
		msg := "Error marshalling new ring to JSON: " + err.Error()
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	logMsg = "Creating new ring in namespace " + newRing.Metadata.Namespace
	kubernetes.Log(types.LogLevelInfo, logMsg)

	// 4. Call the host to create the new resource
	result := kubernetes.CreateResource("Ring", newRing.Metadata.Namespace, string(newRingJson))
	if result.IsErr() {
		msg := "Error creating resource: " + *result.Err()
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	kubernetes.Log(types.LogLevelInfo, "Go operator reconciliation complete.")
	return types.ReconcileResultOK()
}

func Serialize() cm.List[byte] {
	// Not implemented for this example
	return cm.List[byte]{}
}

func Deserialize(state cm.List[byte]) {
	// Not implemented
}

// main is required for the `wasi` target, even if it isn't used.
func main() {}
