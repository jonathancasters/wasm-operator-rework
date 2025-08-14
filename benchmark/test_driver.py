import argparse
import csv
import os
import time

from kubernetes import client, config, watch


def main():
    parser = argparse.ArgumentParser(description="Run a benchmark test for a chain of operators.")
    parser.add_argument("--operator-count", type=int, required=True, help="The number of operators in the ring.")
    parser.add_argument("--active-duration", type=int, required=True, help="Duration of the active phase in seconds.")
    parser.add_argument("--idle-duration", type=int, required=True, help="Duration of the idle phase in seconds.")
    parser.add_argument("--output-dir", type=str, required=True, help="Directory to save the latency results.")
    parser.add_argument("--run-number", type=int, required=True, help="The current run number.")
    args = parser.parse_args()

    print(f"--- Test Driver Started: Operators={args.operator_count}, Run={args.run_number} ---")

    # Load Kubernetes configuration
    try:
        config.load_kube_config()
    except config.ConfigException:
        config.load_incluster_config()

    api = client.CustomObjectsApi()
    crd_api = client.ApiextensionsV1Api()

    # Define the custom resource details
    CRD_GROUP = "ring.benchmark.com"
    CRD_VERSION = "v1"
    CRD_PLURAL = "testresources"
    CRD_NAME = f"{CRD_PLURAL}.{CRD_GROUP}"

    # Wait for the CRD to be established
    print(f"Waiting for CRD '{CRD_NAME}' to be established...")
    while True:
        try:
            crd = crd_api.read_custom_resource_definition(name=CRD_NAME)
            if any(c.status == 'True' and c.type == 'Established' for c in crd.status.conditions):
                print("CRD is established.")
                break
        except client.ApiException as e:
            if e.status == 404:
                pass  # Not found yet
            else:
                raise
        time.sleep(1)

    # Create the initial resource in ns-1 to kick things off
    initial_resource_name = "the-resource"
    initial_namespace = "ns-1"
    final_namespace = "ns-final"

    print(f"Creating initial TestResource in namespace '{initial_namespace}'...")
    api.create_namespaced_custom_object(
        group=CRD_GROUP,
        version=CRD_VERSION,
        namespace=initial_namespace,
        plural=CRD_PLURAL,
        body={
            "apiVersion": f"{CRD_GROUP}/{CRD_VERSION}",
            "kind": "TestResource",
            "metadata": {"name": initial_resource_name},
            "spec": {"nonce": "initial"}
        }
    )
    time.sleep(2)  # Give it a moment to settle

    # --- Active Phase ---
    print(f"--- Starting Active Phase ({args.active_duration}s) ---")
    latencies = []
    active_phase_end_time = time.time() + args.active_duration
    run_count = 0

    while time.time() < active_phase_end_time:
        run_count += 1
        nonce = str(time.time_ns())  # Unique ID for this specific run

        w = watch.Watch()
        stream = w.stream(
            api.list_namespaced_custom_object,
            group=CRD_GROUP,
            version=CRD_VERSION,
            namespace=final_namespace,
            plural=CRD_PLURAL,
            field_selector=f"metadata.name={initial_resource_name}"
        )

        print(f"Ring Run #{run_count}: Triggering with nonce {nonce}...")
        start_time = time.perf_counter()

        # Trigger the ring by patching the first resource
        api.patch_namespaced_custom_object(
            group=CRD_GROUP,
            version=CRD_VERSION,
            namespace=initial_namespace,
            plural=CRD_PLURAL,
            name=initial_resource_name,
            body={"spec": {"nonce": nonce}}
        )

        for event in stream:
            if event['type'] == 'MODIFIED':
                resource = event['object']
                if resource.get('spec', {}).get('nonce') == nonce:
                    end_time = time.perf_counter()
                    latency_ms = (end_time - start_time) * 1000
                    latencies.append(latency_ms)
                    print(f"Ring Run #{run_count}: Completed. Latency: {latency_ms:.2f} ms")
                    w.stop()
                    break

    print(f"--- Active Phase Complete. Total runs: {run_count} ---")

    # --- Idle Phase ---
    print(f"--- Starting Idle Phase ({args.idle_duration}s) ---")
    time.sleep(args.idle_duration)
    print("--- Idle Phase Complete ---")

    # --- Data Output ---
    output_file = f"{args.output_dir}/latency_N{args.operator_count}_run{args.run_number}.csv"
    print(f"Writing {len(latencies)} latency measurements to {output_file}")
    os.makedirs(args.output_dir, exist_ok=True)
    with open(output_file, "w", newline="") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow(["operator_count", "run_number", "latency_ms"])
        for latency in latencies:
            writer.writerow([args.operator_count, args.run_number, latency])

    print(f"--- Test Driver Finished Successfully ---")


if __name__ == "__main__":
    main()