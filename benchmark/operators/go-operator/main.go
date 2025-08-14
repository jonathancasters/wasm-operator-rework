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

// Structs for parsing the TestResource
type TestResource struct {
	ApiVersion string     `json:"apiVersion"`
	Kind       string     `json:"kind"`
	Metadata   ObjectMeta `json:"metadata"`
	Spec       Spec       `json:"spec"`
}

type ObjectMeta struct {
	Name      string `json:"name"`
	Namespace string `json:"namespace"`
}

type Spec struct {
	Nonce string `json:"nonce"`
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
		if pair[0] == "WATCH_NAMESPACE" {
			ns = pair[1]
			break
		}
	}

	if ns == "" {
		kubernetes.Log(types.LogLevelError, "WATCH_NAMESPACE environment variable not set")
		return cm.List[types.WatchRequest]{}
	}

	return cm.ToList([]types.WatchRequest{
		{Kind: "TestResource", Namespace: ns},
	})
}

func Reconcile(req kubeoperator.ReconcileRequest) kubeoperator.ReconcileResult {
	// 1. Get ACTION_NAMESPACE from environment
	action_ns := ""
	for _, pair := range environment.GetEnvironment().Slice() {
		if pair[0] == "ACTION_NAMESPACE" {
			action_ns = pair[1]
			break
		}
	}
	if action_ns == "" {
		msg := "ACTION_NAMESPACE environment variable not set"
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	// 2. Parse the incoming resource
	var resource TestResource
	err := json.Unmarshal([]byte(req.ResourceJSON), &resource)
	if err != nil {
		msg := "Error parsing resource JSON: " + err.Error()
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	// 3. Construct the resource to be applied in the action namespace
	//    The host will use a server-side apply, which handles both creation and updates.
	resourceToApply := TestResource{
		ApiVersion: resource.ApiVersion,
		Kind:       resource.Kind,
		Metadata: ObjectMeta{
			Name:      resource.Metadata.Name,
			Namespace: action_ns, // This is ignored by server-side apply but good practice
		},
		Spec: resource.Spec, // Propagate the nonce
	}

	applyJson, err := json.Marshal(resourceToApply)
	if err != nil {
		msg := "Error marshalling resource to JSON: " + err.Error()
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	// 4. Call UpdateResource to perform a server-side apply.
	updateResult := kubernetes.UpdateResource("TestResource", resource.Metadata.Name, action_ns, string(applyJson))
	if updateResult.IsErr() {
		msg := "Error upserting resource: " + *updateResult.Err()
		kubernetes.Log(types.LogLevelError, msg)
		return types.ReconcileResultError(msg)
	}

	return types.ReconcileResultOK()
}

func Serialize() cm.List[byte] {
	return cm.List[byte]{}
}

func Deserialize(state cm.List[byte]) {}

func main() {}