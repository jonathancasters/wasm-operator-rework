package v1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// RingSpec defines the desired state of Ring
type RingSpec struct {
	Replicas int32    `json:"replicas,omitempty"`
	Nonce    string   `json:"nonce,omitempty"`
	Chain    []string `json:"chain,omitempty"`
	Hop      int32    `json:"hop,omitempty"`
}

//+kubebuilder:object:root=true

// Ring is the Schema for the rings API
type Ring struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   RingSpec   `json:"spec,omitempty"`
}

//+kubebuilder:object:root=true

// RingList contains a list of Ring
type RingList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []Ring `json:"items"`
}

func init() {
	SchemeBuilder.Register(&Ring{}, &RingList{})
}