//go:generate go run go.bytecodealliance.org/cmd/wit-bindgen-go generate --world operator --out internal ./wit

package main

import (
	"encoding/json"
	"fmt"
	"go.bytecodealliance.org/cm"
	"hellokubernetes/internal/wasm-operator/operator/child-api"
	"hellokubernetes/internal/wasm-operator/operator/k8s-http"
	parentapi "hellokubernetes/internal/wasm-operator/operator/parent-api"
)

// Structs to match the Kubernetes API response

type PodList struct {
	Items []Pod `json:"items"`
}

type Pod struct {
	Metadata Metadata `json:"metadata"`
}

type Metadata struct {
	Name string `json:"name"`
}

func init() {
	childapi.Exports.Start = Start
}

// Start is called by the host to initiate the process of sending a request.
func Start() {
	request := k8shttp.Request{
		Method:  k8shttp.MethodGet,
		URI:     "/api/v1/pods",
		Headers: cm.ToList([]k8shttp.Header{}),
		Body:    cm.ToList([]byte{}),
	}

	// Send the request to the parent.
	result := parentapi.SendRequest(request)

	if result.IsErr() {
		// Some form of logging. In WASI, this might go to stderr.
		fmt.Printf("child-component: failed to send request: %s", *result.Err())
		return
	}

	future := result.OK()
	responseResult := future.Get()
	if responseResult.IsErr() {
		fmt.Printf("child-component: failed to get response: %s", *responseResult.Err())
		return
	}

	response := responseResult.OK()
	var podList PodList
	err := json.Unmarshal(response.Body.Bytes.Slice(), &podList)
	if err != nil {
		fmt.Printf("child-component: failed to unmarshal json: %s", err)
		return
	}

	fmt.Printf("Running pods:\n")
	for _, pod := range podList.Items {
		fmt.Printf("- %s\n", pod.Metadata.Name)
	}
}

func main() {}
