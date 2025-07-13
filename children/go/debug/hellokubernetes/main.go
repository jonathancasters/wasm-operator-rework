
package main

import (
	"fmt"
	"example.com/hellokubernetes/wasm-operator/operator/child-api"
	"example.com/hellokubernetes/wasm-operator/operator/parent-api"
	"example.com/hellokubernetes/wasm-operator/operator/http"
	"go.bytecodealliance.org/cm"
)

func main() {
	request := http.Request{
		Method: http.MethodGet,
		URI:    "/api/v1/pods",
		Headers: cm.ToList([]http.Header{{Name: "Accept", Value: "application/json"}}),
		Body:    cm.ToList([]uint8{}),
	}
	
	result := parentapi.SendRequest(request)
	if result.IsErr() {
		fmt.Printf("Error sending request: %v", result.Err())
	}
}

//export HandleResponse
func HandleResponse(response childapi.Response) {
	fmt.Printf("child module received a response: %v", response)
}

