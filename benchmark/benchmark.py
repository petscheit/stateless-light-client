import subprocess
import re
import time
import csv
import os
import sys
from datetime import datetime

# --- Configuration ---
COMMAND = ["cargo", "run", "-r", "--bin", "cli", "prove", "recursive-epoch", "-f", "31"]
CSV_FILE = "bankai_metrics.csv"
SLEEP_INTERVAL = 120  # 2 minutes

CSV_HEADERS = [
    'timestamp', 'atlantic_id', 'epoch', 'n_steps', 'n_memory_holes',
    'range_check', 'poseidon', 'bitwise', 'add_mod', 'range_check96',
    'mul_mod', 'output', 'pedersen'
]

# --- Regular Expressions for Parsing ---
# Using re.compile for efficiency

# For the "in-progress" case
IN_PROGRESS_RE = re.compile(r"Proof not ready yet \(status: IN_PROGRESS\)")

# For successful run data extraction
ATLANTIC_ID_RE = re.compile(r"Proof submitted to Atlantic with ID: (\S+)")

# --- MODIFIED PART START ---
# This now looks for either "Genesis ... Epoch:" or "Recursive ... Target Epoch:"
# This was the main cause of the issue.
EPOCH_RE = re.compile(r"(?:Genesis proof details - Epoch|Recursive epoch proof details - Target Epoch): (\d+)")
# --- MODIFIED PART END ---

EXECUTION_RESOURCES_LINE_RE = re.compile(r"Ok\(ExecutionResources \{ .*? \}\)")

def parse_execution_resources(line):
    """Parses the ExecutionResources string to extract all key-value pairs."""
    resources = {}
    matches = re.findall(r'(\w+): (\d+)', line)
    for key, value in matches:
        if key in CSV_HEADERS:
            resources[key] = int(value)
    return resources

def append_to_csv(data_dict, filename):
    """Appends a dictionary of data to a CSV file. Creates the file and header if it doesn't exist."""
    file_exists = os.path.isfile(filename)
    with open(filename, mode='a', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=CSV_HEADERS, lineterminator=os.linesep)
        if not file_exists:
            writer.writeheader()
        writer.writerow(data_dict)

def run_and_parse():
    """Executes the command, parses the output, and logs to CSV if successful."""
    print(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] --- Running command: {' '.join(COMMAND)} ---")

    try:
        result = subprocess.run(
            COMMAND, capture_output=True, text=True, check=False, timeout=300
        )
        log_output = result.stdout + result.stderr

        if IN_PROGRESS_RE.search(log_output):
            print("⏳ Previous run is still in progress. Skipping this cycle.")
            return

        atlantic_id_match = ATLANTIC_ID_RE.search(log_output)
        epoch_match = EPOCH_RE.search(log_output) # Using the new, more flexible regex
        exec_resources_line_match = EXECUTION_RESOURCES_LINE_RE.search(log_output)

        if atlantic_id_match and epoch_match and exec_resources_line_match:
            print("✅ Run completed successfully. Extracting data...")
            
            atlantic_id = atlantic_id_match.group(1)
            epoch = int(epoch_match.group(1))

            exec_resources_line = exec_resources_line_match.group(0)
            resources = parse_execution_resources(exec_resources_line)
            
            data_row = {'timestamp': datetime.now().isoformat(), 'atlantic_id': atlantic_id, 'epoch': epoch, **resources}
            
            for header in CSV_HEADERS:
                if header not in data_row:
                    data_row[header] = None

            append_to_csv(data_row, CSV_FILE)
            print(f"📈 Data successfully saved to {CSV_FILE}")
        else:
            print("⚠️  Run finished, but key information was not found in the log.")
            # Only print the full log if we failed to parse, to avoid clutter
            print("--- Log Output for Debugging ---")
            print(log_output)
            print("--------------------------------")

    except subprocess.TimeoutExpired:
        print("❌ Command timed out. It will be tried again in the next cycle.")
    except Exception as e:
        print(f"An unexpected error occurred: {e}")

def main():
    """Main loop to run the monitoring task."""
    try:
        while True:
            run_and_parse()
            print(f"--- Waiting for {SLEEP_INTERVAL} seconds... (Press Ctrl+C to stop) ---")
            time.sleep(SLEEP_INTERVAL)
    except KeyboardInterrupt:
        print("\n🛑 Script stopped by user. Exiting.")
        sys.exit(0)

if __name__ == "__main__":
    main()