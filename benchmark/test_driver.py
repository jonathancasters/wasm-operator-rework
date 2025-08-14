import argparse
import csv
import os
import time
import logging
import requests
from tqdm import tqdm

from kubernetes import client, config, watch

PROMETHEUS_URL = "http://localhost:9090"

def get_memory_usage():
    """Queries Prometheus for memory usage of the parent-operator container."""
    query = 'sum(container_memory_working_set_bytes{namespace="default",container="parent-operator"})'
    try:
        response = requests.get(f"{PROMETHEUS_URL}/api/v1/query", params={"query": query})
        response.raise_for_status()
        result = response.json()['data']['result']
        if result:
            return int(result[0]['value'][1])
    except requests.exceptions.RequestException as e:
        logging.error(f"Error querying Prometheus: {e}")
    except (KeyError, IndexError):
        logging.warning("Could not parse Prometheus response.")
    return 0

def write_header_if_needed(file_path, headers):
    """Writes the header to a CSV file if it doesn't exist or is empty."""
    if not os.path.exists(file_path) or os.path.getsize(file_path) == 0:
        with open(file_path, 'w', newline='') as csvfile:
            writer = csv.writer(csvfile)
            writer.writerow(headers)

def main():
    parser = argparse.ArgumentParser(description="Run a benchmark test for a chain of operators.")
    parser.add_argument("--operator-count", type=int, required=True, help="The number of operators in the ring.")
    parser.add_argument("--active-duration", type=int, required=True, help="Duration of the active phase in seconds.")
    parser.add_argument("--idle-duration", type=int, required=True, help="Duration of the idle phase in seconds.")
    parser.add_argument("--latency-file", type=str, required=True, help="File to save the latency results.")
    parser.add_argument("--memory-file", type=str, required=True, help="File to save the memory results.")
    parser.add_argument("--run-number", type=int, required=True, help="The current run number.")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format='%(asctime)s %(message)s', datefmt='%H:%M:%S')

    logging.info(f"🐍 Test Driver Started: Operators={args.operator_count}, Run={args.run_number}")

    # Write headers if needed
    write_header_if_needed(args.latency_file, ["operator_count", "run_number", "latency_ms"])
    write_header_if_needed(args.memory_file, ["operator_count", "run_number", "phase", "timestamp", "memory_bytes"])

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
    logging.info(f"⏳ Waiting for CRD '{CRD_NAME}' to be established...")
    while True:
        try:
            crd = crd_api.read_custom_resource_definition(name=CRD_NAME)
            if any(c.status == 'True' and c.type == 'Established' for c in crd.status.conditions):
                logging.info("✅ CRD is established.")
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

    logging.info(f"🎯 Creating initial TestResource in namespace '{initial_namespace}'...")
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
    logging.info(f"⏱️ Starting Active Phase...")
    latencies = []
    run_count = 0
    with tqdm(total=args.active_duration, desc="Active Phase", unit="s") as pbar:
        start_time = time.time()
        while time.time() < start_time + args.active_duration:
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

            trigger_start_time = time.perf_counter()
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
                        latency_ms = (end_time - trigger_start_time) * 1000
                        latencies.append(latency_ms)
                        pbar.set_postfix({"last_latency_ms": f"{latency_ms:.2f}", "runs": run_count})
                        w.stop()
                        break
            pbar.n = int(time.time() - start_time)
            pbar.refresh()

    logging.info(f"--- Active Phase Complete. Total runs: {run_count} ---")
    active_memory = get_memory_usage()

    # --- Idle Phase ---
    logging.info(f"--- Starting Idle Phase...")
    with tqdm(total=args.idle_duration, desc="Idle Phase", unit="s") as pbar:
        for i in range(args.idle_duration):
            time.sleep(1)
            pbar.update(1)

    logging.info("--- Idle Phase Complete ---")
    idle_memory = get_memory_usage()

    # --- Data Output ---
    logging.info(f"📝 Writing {len(latencies)} latency measurements to {args.latency_file}")
    with open(args.latency_file, "a", newline="") as csvfile:
        writer = csv.writer(csvfile)
        for latency in latencies:
            writer.writerow([args.operator_count, args.run_number, latency])

    logging.info(f"📝 Writing memory measurements to {args.memory_file}")
    with open(args.memory_file, "a", newline="") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow([args.operator_count, args.run_number, "active", int(time.time()), active_memory])
        writer.writerow([args.operator_count, args.run_number, "idle", int(time.time()), idle_memory])

    logging.info(f"🎉 Test Driver Finished Successfully!")


if __name__ == "__main__":
    main()