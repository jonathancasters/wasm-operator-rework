package main

import (
	"context"
	"fmt"
	"os"

	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"

	examplev1 "example.com/ring-operator-bare-metal-go/api/v1"
)

// RingReconciler reconciles a Ring object
type RingReconciler struct {
	client.Client
	Scheme       *runtime.Scheme
	OperatorName string
}

//+kubebuilder:rbac:groups=example.com,resources=rings,verbs=get;list;watch;create;update;patch;delete

func (r *RingReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := ctrl.Log.WithValues("ring", req.NamespacedName)

	var ring examplev1.Ring
	if err := r.Get(ctx, req.NamespacedName, &ring); err != nil {
		log.Error(err, "unable to fetch Ring")
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}

	if len(ring.Spec.Chain) == 0 {
		log.Info("Chain is empty, stopping reconciliation")
		return ctrl.Result{}, nil
	}

	if ring.Spec.Chain[0] == r.OperatorName {
		log.Info("Operator is the current link in the chain")

		newSpec := ring.Spec.DeepCopy()
		newSpec.Hop++
		newSpec.Chain = newSpec.Chain[1:]

		if len(newSpec.Chain) > 0 {
			nextNamespace := newSpec.Chain[0]
			newRing := &examplev1.Ring{
				ObjectMeta: metav1.ObjectMeta{
					Name:      ring.Name,
					Namespace: nextNamespace,
				},
				Spec: *newSpec,
			}

			log.Info("Creating next ring in namespace", "namespace", nextNamespace)
			if err := r.Create(ctx, newRing); err != nil {
				log.Error(err, "unable to create next Ring")
				return ctrl.Result{}, err
			}
		}
	}

	return ctrl.Result{}, nil
}

func main() {
	opts := zap.Options{
		Development: true,
	}
	ctrl.SetLogger(zap.New(zap.UseFlagOptions(&opts)))

	operatorNamespace := os.Getenv("OPERATOR_NAMESPACE")

	mgr, err := ctrl.NewManager(ctrl.GetConfigOrDie(), ctrl.Options{
		Namespace: operatorNamespace,
	})
	if err != nil {
		fmt.Println(err, "unable to start manager")
		os.Exit(1)
	}

	examplev1.AddToScheme(mgr.GetScheme())

	if err = (&RingReconciler{
		Client:       mgr.GetClient(),
		Scheme:       mgr.GetScheme(),
		OperatorName: os.Getenv("OPERATOR_NAME"),
	}).SetupWithManager(mgr); err != nil {
		fmt.Println(err, "unable to create controller")
		os.Exit(1)
	}

	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		fmt.Println(err, "problem running manager")
		os.Exit(1)
	}
}
