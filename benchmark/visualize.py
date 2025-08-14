
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
import os
import sys
import argparse
import glob

def load_data(root_dir):
    """
    Loads all latency and memory data from the structured subdirectories.
    """
    scenarios = ['mixed', 'rust-only', 'go-only']
    latency_dfs = []
    memory_dfs = []

    for scenario in scenarios:
        scenario_dir = os.path.join(root_dir, scenario)
        if not os.path.isdir(scenario_dir):
            continue

        latency_file = os.path.join(scenario_dir, 'latency.csv')
        memory_file = os.path.join(scenario_dir, 'memory.csv')

        if os.path.exists(latency_file):
            df = pd.read_csv(latency_file)
            df['scenario'] = scenario
            latency_dfs.append(df)

        if os.path.exists(memory_file):
            df = pd.read_csv(memory_file)
            df['scenario'] = scenario
            memory_dfs.append(df)

    if not latency_dfs and not memory_dfs:
        print("No data found.")
        return pd.DataFrame(), pd.DataFrame()

    latency_df = pd.concat(latency_dfs, ignore_index=True) if latency_dfs else pd.DataFrame()
    memory_df = pd.concat(memory_dfs, ignore_index=True) if memory_dfs else pd.DataFrame()

    return latency_df, memory_df

def aggregate_data(latency_df, memory_df):
    """Aggregates the raw data for plotting."""
    if not memory_df.empty:
        memory_df['memory_mib'] = memory_df['memory_bytes'] / (1024 * 1024)
        memory_agg = memory_df.groupby(['scenario', 'operator_count', 'phase']).quantile(0.95).reset_index()
    else:
        memory_agg = pd.DataFrame()

    if not latency_df.empty:
        latency_df['latency_s'] = latency_df['latency_ms'] / 1000
    
    return latency_df, memory_agg

def plot_memory_active_vs_idle(memory_agg):
    """Plots memory usage for active vs. idle states in the mixed scenario."""
    if memory_agg.empty:
        print("Skipping memory active vs. idle plot: no memory data.")
        return

    plt.figure(figsize=(10, 6))
    sns.set_theme(style="whitegrid")
    
    mixed_data = memory_agg[memory_agg['scenario'] == 'mixed']

    sns.regplot(x='operator_count', y='memory_mib', data=mixed_data[mixed_data['phase'] == 'active'], label='Active', n_boot=1000)
    sns.regplot(x='operator_count', y='memory_mib', data=mixed_data[mixed_data['phase'] == 'idle'], label='Idle', n_boot=1000)

    plt.title('Memory Usage Scaling: Active vs. Idle States (Mixed Operators)')
    plt.xlabel('Number of Operators')
    plt.ylabel('Memory Usage (MiB)')
    plt.legend()
    plt.savefig('memory_active_vs_idle.png', dpi=300)
    plt.close()
    print("Generated memory_active_vs_idle.png")

def plot_language_impact(latency_df, memory_agg):
    """Plots the impact of operator language on performance."""
    if memory_agg.empty and latency_df.empty:
        print("Skipping language impact plot: no data.")
        return

    # Memory Plot
    if not memory_agg.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")
        
        lang_data = memory_agg[memory_agg['scenario'].isin(['rust-only', 'go-only']) & (memory_agg['phase'] == 'active')]
        
        sns.regplot(x='operator_count', y='memory_mib', data=lang_data[lang_data['scenario'] == 'rust-only'], label='Rust', n_boot=1000)
        sns.regplot(x='operator_count', y='memory_mib', data=lang_data[lang_data['scenario'] == 'go-only'], label='Go', n_boot=1000)

        plt.title('Framework Memory Usage by Operator Language (Active Phase)')
        plt.xlabel('Number of Operators')
        plt.ylabel('Memory Usage (MiB)')
        plt.legend()
        plt.savefig('memory_rust_vs_go.png', dpi=300)
        plt.close()
        print("Generated memory_rust_vs_go.png")

    # Latency Plot
    if not latency_df.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")

        lang_data = latency_df[latency_df['scenario'].isin(['rust-only', 'go-only'])]

        sns.lineplot(x='operator_count', y='latency_s', data=lang_data, hue='scenario', style='scenario', markers=True, dashes=False, errorbar=('ci', 95))

        plt.title('Framework End-to-End Latency by Operator Language')
        plt.xlabel('Number of Operators')
        plt.ylabel('End-to-End Latency (s)')
        plt.legend(title='Scenario')
        plt.savefig('latency_rust_vs_go.png', dpi=300)
        plt.close()
        print("Generated latency_rust_vs_go.png")

def plot_comprehensive_scalability(latency_df, memory_agg):
    """Plots comprehensive scalability analysis."""
    if memory_agg.empty and latency_df.empty:
        print("Skipping comprehensive scalability plot: no data.")
        return

    # Memory Plot
    if not memory_agg.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")

        active_data = memory_agg[memory_agg['phase'] == 'active'].copy()
        active_data['scenario_phase'] = active_data['scenario'] + '-' + active_data['phase']

        sns.regplot(x='operator_count', y='memory_mib', data=active_data[active_data['scenario'] == 'mixed'], label='Mixed-Active', n_boot=1000)
        sns.regplot(x='operator_count', y='memory_mib', data=active_data[active_data['scenario'] == 'rust-only'], label='Rust-Active', n_boot=1000)
        sns.regplot(x='operator_count', y='memory_mib', data=active_data[active_data['scenario'] == 'go-only'], label='Go-Active', n_boot=1000)

        plt.title('Comprehensive Memory Scalability Analysis (Active Phase)')
        plt.xlabel('Number of Operators')
        plt.ylabel('Memory Usage (MiB)')
        plt.legend()
        plt.savefig('memory_scalability_all.png', dpi=300)
        plt.close()
        print("Generated memory_scalability_all.png")

    # Latency Plot
    if not latency_df.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")

        sns.lineplot(x='operator_count', y='latency_s', data=latency_df, hue='scenario', style='scenario', markers=True, dashes=False, errorbar=('ci', 95))

        plt.title('Comprehensive Latency Scalability Analysis')
        plt.xlabel('Number of Operators')
        plt.ylabel('End-to-End Latency (s)')
        plt.legend(title='Scenario')
        plt.savefig('latency_scalability_all.png', dpi=300)
        plt.close()
        print("Generated latency_scalability_all.png")


def main():
    """Main function to run the analysis."""
    parser = argparse.ArgumentParser(description='Generate plots for benchmark results.')
    parser.add_argument('results_dir', type=str, help='Root directory of the benchmark results.')
    args = parser.parse_args()

    if not os.path.isdir(args.results_dir):
        print(f"Error: Directory '{args.results_dir}' not found.")
        sys.exit(1)

    latency_df, memory_df = load_data(args.results_dir)

    if latency_df.empty and memory_df.empty:
        return

    latency_agg, memory_agg = aggregate_data(latency_df, memory_df)

    plot_memory_active_vs_idle(memory_agg)
    plot_language_impact(latency_agg, memory_agg)
    plot_comprehensive_scalability(latency_agg, memory_agg)

    print("Plots generated successfully in the current directory.")

if __name__ == '__main__':
    main()
