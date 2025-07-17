package main

import (
	"fmt"
	"go.bytecodealliance.org/cm"
	childapi "hellokubernetes/internal/wasm-operator/operator/child-api"
	"hellokubernetes/internal/wasm-operator/operator/http"
	parent "hellokubernetes/internal/wasm-operator/operator/parent-api"
	"hellokubernetes/internal/wasm-operator/operator/types"
)

func main() {
	// Create a request to get all pods from the Kubernetes API server.
	request := http.Request{
		Method:  http.MethodGet,
		URI:     "/api/v1/pods",
		Headers: cm.ToList([]http.Header{}),
		Body:    cm.ToList([]byte{}),
	}

	// Send the request to the parent.
	result := parent.SendRequest(request)
	if result.IsErr() {
		// Some form of logging. In WASI, this might go to stderr.
		fmt.Printf("failed to send request: %s\n", *result.Err())
		return
	}

	// The request was sent, the async ID is in result.OK().
	// The host will call HandleResponse with the response.
	fmt.Printf("Sent request with ID: %d\n", *result.OK())
}

func init() {
	childapi.Exports.HandleResponse = HandleResponse
}

// HandleResponse is called by the host when a response is ready.
func HandleResponse(id types.AsyncID, res http.Response) {
	fmt.Printf("Received response for ID: %d\n", id)
	fmt.Printf("Status: %d\n", res.Status)
	fmt.Printf("Body: %s\n", string(res.Body.Slice()))
}
