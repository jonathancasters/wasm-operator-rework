//go:generate go run go.bytecodealliance.org/cmd/wit-bindgen-go generate --world operator --out internal ./wit

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"time"

	"go.bytecodealliance.org/cm"
	childapi "ring-operator/internal/wasm-operator/operator/child-api"
	k8shttp "ring-operator/internal/wasm-operator/operator/k8s-http"
	parentapi "ring-operator/internal/wasm-operator/operator/parent-api"
)

const compileTimeNonce = "go-ring-operator-v1-20250803"

// TestResourceSpec defines the desired state of TestResource
type TestResourceSpec struct {
	Nonce     int64  `json:"nonce"`
	UpdatedAt string `json:"updated_at,omitempty"`
}

// ObjectMeta contains metadata for a Kubernetes object.
type ObjectMeta struct {
	Name            string `json:"name"`
	Namespace       string `json:"namespace,omitempty"`
	ResourceVersion string `json:"resourceVersion,omitempty"`
}

// TestResource is the Schema for the testresources API
type TestResource struct {
	APIVersion string           `json:"apiVersion"`
	Kind       string           `json:"kind"`
	Metadata   ObjectMeta       `json:"metadata"`
	Spec       TestResourceSpec `json:"spec"`
}

// TestResourceList contains a list of TestResource
type TestResourceList struct {
	Items []TestResource `json:"items"`
}

func init() {
	childapi.Exports.Start = Start
}

func main() {}

// Start is the entry point called by the host.
func Start() {
	fmt.Printf("Starting Go ring operator reconciliation. Compile-time nonce: %s\n", compileTimeNonce)

	inNamespace := os.Getenv("IN_NAMESPACE")
	outNamespace := os.Getenv("OUT_NAMESPACE")
	if inNamespace == "" || outNamespace == "" {
		fmt.Println("Error: IN_NAMESPACE and OUT_NAMESPACE environment variables must be set.")
		return
	}

	fmt.Printf("IN_NAMESPACE=%s, OUT_NAMESPACE=%s\n", inNamespace, outNamespace)

	// 1. List TestResources in the input namespace
	listURI := fmt.Sprintf("/apis/amurant.io/v1/namespaces/%s/testresources", inNamespace)
	resp, err := sendRequest(k8shttp.MethodGet, listURI, nil)
	if err != nil {
		fmt.Printf("Error listing TestResources in namespace %s: %v\n", inNamespace, err)
		return
	}

	var inResourceList TestResourceList
	if err := json.Unmarshal(resp.Body.Bytes.Slice(), &inResourceList); err != nil {
		fmt.Printf("Error unmarshalling input TestResourceList: %v\n", err)
		return
	}

	fmt.Printf("Found %d resources in %s\n", len(inResourceList.Items), inNamespace)

	// 2. Loop through all input resources and reconcile them
	for _, inResource := range inResourceList.Items {
		reconcileResource(&inResource, outNamespace)
	}
}

func reconcileResource(inResource *TestResource, outNamespace string) {
	resourceName := inResource.Metadata.Name
	fmt.Printf("Reconciling resource: %s\n", resourceName)

	// 2. Get the corresponding TestResource in the output namespace
	getURI := fmt.Sprintf("/apis/amurant.io/v1/namespaces/%s/testresources/%s", outNamespace, resourceName)
	outResp, err := sendRequest(k8shttp.MethodGet, getURI, nil)

	if err != nil {
		// If the error is a 404 Not Found, create the resource.
		if err.Error() == "404" {
			fmt.Printf("Output resource %s not found, creating it.\n", resourceName)
			createResource(inResource, outNamespace)
		} else {
			fmt.Printf("Error getting output resource %s: %v\n", resourceName, err)
		}
		return
	}

	// 3. If the resource exists, compare nonces and update if necessary.
	var outResource TestResource
	if err := json.Unmarshal(outResp.Body.Bytes.Slice(), &outResource); err != nil {
		fmt.Printf("Error unmarshalling output resource %s: %v\n", resourceName, err)
		return
	}

	if inResource.Spec.Nonce > outResource.Spec.Nonce {
		fmt.Printf("Input nonce (%d) > output nonce (%d) for %s. Updating.\n", inResource.Spec.Nonce, outResource.Spec.Nonce, resourceName)
		updateResource(inResource, &outResource, outNamespace)
	} else {
		fmt.Printf("Input nonce (%d) <= output nonce (%d) for %s. No action needed.\n", inResource.Spec.Nonce, outResource.Spec.Nonce, resourceName)
	}
}

func createResource(inResource *TestResource, outNamespace string) {
	newResource := TestResource{
		APIVersion: "amurant.io/v1",
		Kind:       "TestResource",
		Metadata: ObjectMeta{
			Name:      inResource.Metadata.Name,
			Namespace: outNamespace,
		},
		Spec: TestResourceSpec{
			Nonce:     inResource.Spec.Nonce,
			UpdatedAt: time.Now().UTC().Format(time.RFC3339Nano),
		},
	}

	body, err := json.Marshal(newResource)
	if err != nil {
		fmt.Printf("Error marshalling for create: %v\n", err)
		return
	}

	createURI := fmt.Sprintf("/apis/amurant.io/v1/namespaces/%s/testresources", outNamespace)
	_, err = sendRequest(k8shttp.MethodPost, createURI, body)
	if err != nil {
		fmt.Printf("Error creating resource %s: %v\n", newResource.Metadata.Name, err)
	} else {
		fmt.Printf("Successfully created resource %s.\n", newResource.Metadata.Name)
	}
}

func updateResource(inResource, outResource *TestResource, outNamespace string) {
	outResource.Spec.Nonce = inResource.Spec.Nonce
	outResource.Spec.UpdatedAt = time.Now().UTC().Format(time.RFC3339Nano)

	body, err := json.Marshal(outResource)
	if err != nil {
		fmt.Printf("Error marshalling for update: %v\n", err)
		return
	}

	updateURI := fmt.Sprintf("/apis/amurant.io/v1/namespaces/%s/testresources/%s", outNamespace, outResource.Metadata.Name)
	_, err = sendRequest(k8shttp.MethodPut, updateURI, body)
	if err != nil {
		fmt.Printf("Error updating resource %s: %v\n", outResource.Metadata.Name, err)
	} else {
		fmt.Printf("Successfully updated resource %s.\n", outResource.Metadata.Name)
	}
}

// sendRequest is a helper to communicate with the parent host.
func sendRequest(method k8shttp.Method, uri string, body []byte) (*k8shttp.Response, error) {
	headers := cm.ToList([]k8shttp.Header{
		{Name: "Content-Type", Value: "application/json"},
	})

	request := k8shttp.Request{
		Method:  method,
		URI:     uri,
		Headers: headers,
		Body:    cm.ToList(body),
	}

	result := parentapi.SendRequest(request)
	if result.IsErr() {
		return nil, fmt.Errorf("failed to send request: %s", *result.Err())
	}

	future := result.OK()
	responseResult := future.Get()
	if responseResult.IsErr() {
		return nil, fmt.Errorf("failed to get response: %s", *responseResult.Err())
	}

	response := responseResult.OK()

	// The host indicates application-level errors (like 404) via the status code in the body,
	// which we assume is a string for now. A more robust solution would be a structured response.
	// For now, we'll check if the body can be parsed as an integer status code.
	bodyStr := string(response.Body.Bytes.Slice())
	if code, err := strconv.Atoi(bodyStr); err == nil && code >= 400 {
		return nil, fmt.Errorf("%d", code)
	}

	return response, nil
}
