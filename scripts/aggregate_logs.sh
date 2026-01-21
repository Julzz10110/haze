#!/bin/bash
# Aggregate logs from multiple HAZE nodes for analysis

set -e

LOG_DIR="${1:-.}"
OUTPUT="${2:-aggregated.log}"

echo "HAZE Log Aggregator"
echo "==================="
echo "Source directory: $LOG_DIR"
echo "Output file: $OUTPUT"
echo ""

# Clear output file
> "$OUTPUT"

# Find all node log files
LOG_FILES=$(find "$LOG_DIR" -name "node*.log" -type f 2>/dev/null | sort)

if [ -z "$LOG_FILES" ]; then
    echo "No node log files found in $LOG_DIR"
    exit 1
fi

echo "Found log files:"
echo "$LOG_FILES" | while read -r log; do
    echo "  - $log"
done
echo ""

# Aggregate logs
for log in $LOG_FILES; do
    node_name=$(basename "$log" .log)
    echo "=== $node_name ===" >> "$OUTPUT"
    cat "$log" >> "$OUTPUT"
    echo "" >> "$OUTPUT"
done

echo "Logs aggregated to: $OUTPUT"
echo ""
echo "Useful commands:"
echo "  # Extract metrics:"
echo "  grep 'Metrics:' $OUTPUT"
echo ""
echo "  # Extract block events:"
echo "  grep 'Block created:' $OUTPUT"
echo ""
echo "  # Extract errors:"
echo "  grep -i 'error\\|failed\\|warn' $OUTPUT"
echo ""
echo "  # Extract sync events:"
echo "  grep -i 'sync\\|peer' $OUTPUT"
