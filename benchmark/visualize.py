import sys

import argparse
import matplotlib.pyplot as plt
import os
import pandas as pd
import seaborn as sns


def load_data(root_dir):
    """
    Loads all latency and memory data from the structured subdirectories.
    """
    scenarios = ['mixed', 'rust', 'go']
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


def plot_memory_savings(memory_agg, results_dir):
    """
    Plots the memory savings between active and idle states using a bar chart.
    """
    if memory_agg.empty:
        print("Skipping memory savings plot: no memory data.")
        return

    plt.figure(figsize=(12, 7))
    sns.set_theme(style="whitegrid")

    mixed_data = memory_agg[memory_agg['scenario'] == 'mixed']
    if mixed_data.empty:
        print("No mixed scenario data for memory savings plot.")
        return

    # Pivot data to have active and idle as columns
    pivot_df = mixed_data.pivot_table(index='operator_count', columns='phase', values='memory_mib').reset_index()
    pivot_df.rename(columns={'active': 'Active', 'idle': 'Idle'}, inplace=True)

    if 'Active' not in pivot_df.columns or 'Idle' not in pivot_df.columns:
        print("Could not find both 'active' and 'idle' phases in the data for the mixed scenario.")
        return

    # Calculate savings
    pivot_df['Savings (MiB)'] = pivot_df['Active'] - pivot_df['Idle']
    pivot_df['Savings (Percentage)'] = (pivot_df['Savings (MiB)'] / pivot_df['Active']) * 100

    # Melt the DataFrame for easier plotting with seaborn
    plot_df = pivot_df.melt(id_vars=['operator_count'], value_vars=['Active', 'Idle'],
                            var_name='State', value_name='Memory (MiB)')

    # Bar plot for absolute memory usage
    ax = sns.barplot(x='operator_count', y='Memory (MiB)', hue='State', data=plot_df, palette="viridis")

    plt.title('Memory Usage: Active vs. Idle States (Mixed Operators)')
    plt.xlabel('Number of Operators')
    plt.ylabel('P95 Memory Usage (MiB)')
    plt.legend(title='State')

    # Annotate with savings percentage
    for p in ax.patches:
        height = p.get_height()
        if height > 0:
            ax.annotate(f'{height:.1f}', (p.get_x() + p.get_width() / 2., height),
                        ha='center', va='center', fontsize=9, color='black', xytext=(0, 5),
                        textcoords='offset points')

    # Add a secondary axis or a separate plot for savings
    plt.twinx()
    sns.lineplot(x=pivot_df['operator_count'].astype(str), y=pivot_df['Savings (Percentage)'],
                 color='red', marker='o', label='Savings (%)')
    plt.ylabel('Memory Savings (%)')
    plt.legend(loc='upper right')
    plt.ylim(0, 100)

    plt.tight_layout()
    plt.savefig(os.path.join(results_dir, 'memory_savings_comparison.png'), dpi=300)
    plt.close()
    print("Generated memory_savings_comparison.png")


def plot_memory_active_vs_idle(memory_agg, results_dir):
    """Plots memory usage for active vs. idle states in the mixed scenario."""
    if memory_agg.empty:
        print("Skipping memory active vs. idle plot: no memory data.")
        return

    plt.figure(figsize=(10, 6))
    sns.set_theme(style="whitegrid")

    mixed_data = memory_agg[memory_agg['scenario'] == 'mixed']

    sns.regplot(x='operator_count', y='memory_mib', data=mixed_data[mixed_data['phase'] == 'active'], label='Active',
                n_boot=1000)
    sns.regplot(x='operator_count', y='memory_mib', data=mixed_data[mixed_data['phase'] == 'idle'], label='Idle',
                n_boot=1000)

    plt.title('Memory Usage Scaling: Active vs. Idle States (Mixed Operators)')
    plt.xlabel('Number of Operators')
    plt.ylabel('Memory Usage (MiB)')
    plt.legend()
    plt.savefig(os.path.join(results_dir, 'memory_active_vs_idle.png'), dpi=300)
    plt.close()
    print("Generated memory_active_vs_idle.png")


def plot_language_impact(latency_df, memory_agg, results_dir):
    """Plots the impact of operator language on performance."""
    if memory_agg.empty and latency_df.empty:
        print("Skipping language impact plot: no data.")
        return

    # Memory Plot
    if not memory_agg.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")

        lang_data = memory_agg[memory_agg['scenario'].isin(['rust', 'go']) & (memory_agg['phase'] == 'active')]

        sns.regplot(x='operator_count', y='memory_mib', data=lang_data[lang_data['scenario'] == 'rust'], label='Rust',
                    n_boot=1000)
        sns.regplot(x='operator_count', y='memory_mib', data=lang_data[lang_data['scenario'] == 'go'], label='Go',
                    n_boot=1000)

        plt.title('Framework Memory Usage by Operator Language (Active Phase)')
        plt.xlabel('Number of Operators')
        plt.ylabel('Memory Usage (MiB)')
        plt.legend()
        plt.savefig(os.path.join(results_dir, 'memory_rust_vs_go.png'), dpi=300)
        plt.close()
        print("Generated memory_rust_vs_go.png")

    # Latency Plot
    if not latency_df.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")

        lang_data = latency_df[latency_df['scenario'].isin(['rust', 'go'])]

        sns.lineplot(x='operator_count', y='latency_s', data=lang_data, hue='scenario', style='scenario', markers=True,
                     dashes=False, errorbar=('ci', 95))

        plt.title('Framework End-to-End Latency by Operator Language')
        plt.xlabel('Number of Operators')
        plt.ylabel('End-to-End Latency (s)')
        plt.legend(title='Scenario')
        plt.savefig(os.path.join(results_dir, 'latency_rust_vs_go.png'), dpi=300)
        plt.close()
        print("Generated latency_rust_vs_go.png")


def plot_comprehensive_scalability(latency_df, memory_agg, results_dir):
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

        sns.regplot(x='operator_count', y='memory_mib', data=active_data[active_data['scenario'] == 'mixed'],
                    label='Mixed-Active', n_boot=1000)
        sns.regplot(x='operator_count', y='memory_mib', data=active_data[active_data['scenario'] == 'rust'],
                    label='Rust-Active', n_boot=1000)
        sns.regplot(x='operator_count', y='memory_mib', data=active_data[active_data['scenario'] == 'go'],
                    label='Go-Active', n_boot=1000)

        plt.title('Comprehensive Memory Scalability Analysis (Active Phase)')
        plt.xlabel('Number of Operators')
        plt.ylabel('Memory Usage (MiB)')
        plt.legend()
        plt.savefig(os.path.join(results_dir, 'memory_scalability_all.png'), dpi=300)
        plt.close()
        print("Generated memory_scalability_all.png")

    # Latency Plot
    if not latency_df.empty:
        plt.figure(figsize=(10, 6))
        sns.set_theme(style="whitegrid")

        sns.lineplot(x='operator_count', y='latency_s', data=latency_df, hue='scenario', style='scenario', markers=True,
                     dashes=False, errorbar=('ci', 95))

        plt.title('Comprehensive Latency Scalability Analysis')
        plt.xlabel('Number of Operators')
        plt.ylabel('End-to-End Latency (s)')
        plt.legend(title='Scenario')
        plt.savefig(os.path.join(results_dir, 'latency_scalability_all.png'), dpi=300)
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

    plot_memory_active_vs_idle(memory_agg, args.results_dir)
    plot_memory_savings(memory_agg, args.results_dir)
    plot_language_impact(latency_agg, memory_agg, args.results_dir)
    plot_comprehensive_scalability(latency_agg, memory_agg, args.results_dir)

    print(f"Plots generated successfully in {args.results_dir}.")


if __name__ == '__main__':
    main()