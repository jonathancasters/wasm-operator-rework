//go:generate go run go.bytecodealliance.org/cmd/wit-bindgen-go generate --world operator --out internal ./wit

package main

import (
	"fmt"
	"go.bytecodealliance.org/cm"
	"hellokubernetes/internal/wasm-operator/operator/child-api"
	"hellokubernetes/internal/wasm-operator/operator/http"
	parentapi "hellokubernetes/internal/wasm-operator/operator/parent-api"
	"hellokubernetes/internal/wasm-operator/operator/types"
)

func init() {
	childapi.Exports.HandleResponse = HandleResponse
	childapi.Exports.Start = Start
}

// HandleResponse is called by the host when a response is ready.
func HandleResponse(id types.AsyncID, res http.Response) {
	fmt.Printf("Received response for ID: %d\n", id)
	fmt.Printf("Status: %d\n", res.Status)
	fmt.Printf("Body: %s\n", string(res.Body.Slice()))
}

// Start is called by the host to initiate the process of sending a request.
func Start() {
	request := http.Request{
		Method:  http.MethodGet,
		URI:     "/api/v1/pods",
		Headers: cm.ToList([]http.Header{}),
		Body:    cm.ToList([]byte{}),
	}

	// Send the request to the parent.
	result := parentapi.SendRequest(request)

	if result.IsErr() {
		// Some form of logging. In WASI, this might go to stderr.
		fmt.Printf("child-component: failed to send request: %s\n", *result.Err())
	} else {
		// The request was sent, the async ID is in result.OK().
		// The host will call HandleResponse with the response.
		fmt.Printf("child-component: sent request with ID: %d\n", *result.OK())
	}
}

func main() {}
