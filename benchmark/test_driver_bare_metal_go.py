import argparse
import csv
import os
import time
import logging
import subprocess

from kubernetes import client, config, watch

PROMETHEUS_URL = "http://localhost:9090"


def get_memory_usage(container_name):
    """Queries Prometheus for memory usage of the specified container."""
    query = f'sum(container_memory_working_set_bytes{{namespace="default",container="{container_name}"}})'
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
    parser = argparse.ArgumentParser(description="Run a benchmark test for a bare-metal operator.")
    parser.add_argument("--operator-count", type=int, required=True, help="The number of operators in the ring.")
    parser.add_argument("--active-duration", type=int, required=True, help="Duration of the active phase in seconds.")
    parser.add_argument("--idle-duration", type=int, required=True, help="Duration of the idle phase in seconds.")
    parser.add_argument("--latency-file", type=str, required=True, help="File to save the latency results.")
    parser.add_argument("--memory-file", type=str, required=True, help="File to save the memory results.")
    parser.add_argument("--run-number", type=int, required=True, help="The current run number.")
    parser.add_argument("--script-dir", type=str, required=True, help="The directory of the benchmark scripts.")

    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format='%(asctime)s %(levelname)s %(message)s', datefmt='%H:%M:%S')

    logging.info(f"ð Test Driver Started: Operators={args.operator_count}, Run={args.run_number}")

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
    apps_v1 = client.AppsV1Api()
    core_v1 = client.CoreV1Api()

    # Generate and apply manifests
    logging.info("ð Generating and applying manifests...")
    manifest_script = os.path.join(args.script_dir, "operators/go-operator-bare-metal/generate-manifests.sh")
    manifests = subprocess.check_output([manifest_script, str(args.operator_count)])
    with open("generated-manifests.yaml", "wb") as f:
        f.write(manifests)
    
    try:
        subprocess.run(["kubectl", "apply", "-f", "generated-manifests.yaml"], check=True, capture_output=True, text=True)
        logging.info("Manifests applied successfully.")
    except subprocess.CalledProcessError as e:
        logging.error("Failed to apply manifests:")
        logging.error(e.stdout)
        logging.error(e.stderr)
        exit(1)

    # Wait for operators to be ready
    for i in range(1, args.operator_count + 1):
        deployment_name = f"ring-operator-bare-metal-go-{i}"
        logging.info(f"â³ Waiting for deployment '{deployment_name}' to be ready...")
        while True:
            try:
                deployment = apps_v1.read_namespaced_deployment_status(name=deployment_name, namespace=f"operator-{i}")
                status = deployment.status
                if status.ready_replicas is not None and status.ready_replicas == deployment.spec.replicas:
                    logging.info(f"â Deployment '{deployment_name}' is ready with {status.ready_replicas} replica(s).")
                    break
            except client.ApiException as e:
                if e.status == 404:
                    logging.warning(f"Deployment '{deployment_name}' not found. Waiting...")
                else:
                    logging.error(f"Error checking deployment status: {e}")
            time.sleep(2)

    # --- Active Phase ---
    logging.info("â±ï¸ Starting Active Phase...")
    latencies = []
    chain = [f"operator-{i}" for i in range(1, args.operator_count + 1)]
    initial_namespace = chain[0]
    final_namespace = chain[-1]

    with tqdm(total=args.active_duration, desc="Active Phase", unit="s") as pbar:
        start_time = time.time()
        while time.time() < start_time + args.active_duration:
            nonce = str(time.time_ns())
            ring_name = f"ring-{nonce}"

            w = watch.Watch()
            stream = w.stream(
                api.list_namespaced_custom_object,
                group="example.com",
                version="v1",
                namespace=final_namespace,
                plural="rings",
                field_selector=f"metadata.name={ring_name}",
                timeout_seconds=60
            )

            trigger_start_time = time.perf_counter()
            api.create_namespaced_custom_object(
                group="example.com",
                version="v1",
                namespace=initial_namespace,
                plural="rings",
                body={
                    "apiVersion": "example.com/v1",
                    "kind": "Ring",
                    "metadata": {"name": ring_name},
                    "spec": {"replicas": 1, "nonce": nonce, "chain": chain, "hop": 0}
                }
            )

            for event in stream:
                if event['type'] == 'ADDED':
                    resource = event['object']
                    if resource.get('spec', {}).get('nonce') == nonce:
                        end_time = time.perf_counter()
                        latency_ms = (end_time - trigger_start_time) * 1000
                        latencies.append(latency_ms)
                        pbar.set_postfix_str(f"Runs: {len(latencies)}, Latency: {latency_ms:.2f}ms")
                        w.stop()
                        break

            pbar.n = int(time.time() - start_time)
            pbar.refresh()
            time.sleep(0.1)

    logging.info(f"--- Active Phase Complete. Total runs: {len(latencies)} ---")
    active_memory = get_memory_usage("ring-operator-bare-metal-go")

    # --- Idle Phase ---
    logging.info("--- Starting Idle Phase...")
    logging.info(f"ð´ Sleeping for {args.idle_duration} seconds...")
    time.sleep(args.idle_duration)

    logging.info("--- Idle Phase Complete ---")
    idle_memory = get_memory_usage("ring-operator-bare-metal-go")

    # --- Data Output ---
    logging.info(f"ð Writing {len(latencies)} latency measurements to {args.latency_file}")
    with open(args.latency_file, "a", newline="") as csvfile:
        writer = csv.writer(csvfile)
        for latency in latencies:
            writer.writerow([args.operator_count, args.run_number, latency])

    logging.info(f"ð Writing memory measurements to {args.memory_file}")
    with open(args.memory_file, "a", newline="") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow([args.operator_count, args.run_number, "active", int(time.time()), active_memory])
        writer.writerow([args.operator_count, args.run_number, "idle", int(time.time()), idle_memory])

    # --- Cleanup ---
    logging.info("ðï¸ Cleaning up resources...")
    subprocess.run(["kubectl", "delete", "-f", "generated-manifests.yaml"])
    os.remove("generated-manifests.yaml")

    logging.info("ð Test Driver Finished Successfully!")


if __name__ == "__main__":
    main()
