//go:generate go run go.bytecodealliance.org/cmd/wit-bindgen-go generate --world operator --out internal ./wit

package main

import (
	"fmt"
	"go.bytecodealliance.org/cm"
	"hellokubernetes/internal/wasm-operator/operator/child-api"
	"hellokubernetes/internal/wasm-operator/operator/http"
	parentapi "hellokubernetes/internal/wasm-operator/operator/parent-api"
)

func init() {
	childapi.Exports.Start = Start
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
	fmt.Printf("Received response")
	fmt.Printf("Status: %d", response.Status)
	fmt.Printf("Body: %s", string(response.Body.Slice()))
}

func main() {}
