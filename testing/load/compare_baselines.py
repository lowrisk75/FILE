#!/usr/bin/env python3
"""
FILE Relay Server - Baseline Comparison Tool

Compare load test results to detect performance regressions.
"""

import argparse
import json
import sys
from dataclasses import dataclass
from typing import Dict, List, Tuple


@dataclass
class ComparisonResult:
    """Result of comparing two baselines."""
    metric_name: str
    baseline_value: float
    current_value: float
    delta: float
    delta_percent: float
    status: str  # 'pass', 'warn', 'fail'
    threshold_warn: float
    threshold_fail: float


class BaselineComparator:
    """Compare load test baselines and detect regressions."""

    # Regression thresholds (negative = worse performance)
    THRESHOLDS = {
        'registration_success_rate': {'warn': -1.0, 'fail': -5.0},
        'packet_delivery_rate': {'warn': -2.0, 'fail': -5.0},
        'packets_per_second': {'warn': -10.0, 'fail': -20.0},
        'latency_avg': {'warn': 20.0, 'fail': 50.0},  # Positive = worse (higher latency)
        'latency_p95': {'warn': 30.0, 'fail': 70.0},
        'latency_p99': {'warn': 50.0, 'fail': 100.0},
    }

    def __init__(self, baseline_file: str, current_file: str):
        self.baseline_file = baseline_file
        self.current_file = current_file

        with open(baseline_file) as f:
            self.baseline = json.load(f)

        with open(current_file) as f:
            self.current = json.load(f)

    def compare_metric(self, metric_name: str) -> ComparisonResult:
        """Compare a single metric."""
        baseline_value = self.baseline.get(metric_name, 0.0)
        current_value = self.current.get(metric_name, 0.0)

        if baseline_value == 0:
            delta = 0.0
            delta_percent = 0.0
        else:
            delta = current_value - baseline_value
            delta_percent = (delta / baseline_value) * 100

        # Determine status
        thresholds = self.THRESHOLDS.get(metric_name, {'warn': -999, 'fail': -999})

        # For latency, higher is worse
        if 'latency' in metric_name:
            if delta_percent > thresholds['fail']:
                status = 'fail'
            elif delta_percent > thresholds['warn']:
                status = 'warn'
            else:
                status = 'pass'
        else:
            # For other metrics, lower is worse
            if delta_percent < thresholds['fail']:
                status = 'fail'
            elif delta_percent < thresholds['warn']:
                status = 'warn'
            else:
                status = 'pass'

        return ComparisonResult(
            metric_name=metric_name,
            baseline_value=baseline_value,
            current_value=current_value,
            delta=delta,
            delta_percent=delta_percent,
            status=status,
            threshold_warn=thresholds['warn'],
            threshold_fail=thresholds['fail']
        )

    def compare_all(self) -> List[ComparisonResult]:
        """Compare all key metrics."""
        metrics_to_compare = [
            'registration_success_rate',
            'packet_delivery_rate',
            'packets_per_second',
            'latency_avg',
            'latency_p95',
            'latency_p99',
        ]

        results = []
        for metric in metrics_to_compare:
            if metric in self.baseline and metric in self.current:
                results.append(self.compare_metric(metric))

        return results

    def print_comparison(self, results: List[ComparisonResult]) -> None:
        """Print comparison results in human-readable format."""
        print()
        print("=" * 90)
        print("BASELINE COMPARISON")
        print("=" * 90)
        print()
        print(f"Baseline: {self.baseline_file}")
        print(f"Current:  {self.current_file}")
        print()

        # Summary counts
        pass_count = sum(1 for r in results if r.status == 'pass')
        warn_count = sum(1 for r in results if r.status == 'warn')
        fail_count = sum(1 for r in results if r.status == 'fail')

        print(f"Summary: {pass_count} passed, {warn_count} warnings, {fail_count} failures")
        print()

        # Detailed results
        print(f"{'Metric':<30} {'Baseline':>12} {'Current':>12} {'Delta':>10} {'Status':>8}")
        print("-" * 90)

        for result in results:
            # Format values
            if 'rate' in result.metric_name:
                baseline_str = f"{result.baseline_value:.2f}%"
                current_str = f"{result.current_value:.2f}%"
            elif 'latency' in result.metric_name:
                baseline_str = f"{result.baseline_value:.2f} ms"
                current_str = f"{result.current_value:.2f} ms"
            else:
                baseline_str = f"{result.baseline_value:.2f}"
                current_str = f"{result.current_value:.2f}"

            # Delta with sign and color
            delta_str = f"{result.delta_percent:+.2f}%"

            # Status icon
            if result.status == 'pass':
                status_icon = '✓ PASS'
            elif result.status == 'warn':
                status_icon = '⚠ WARN'
            else:
                status_icon = '✗ FAIL'

            print(f"{result.metric_name:<30} {baseline_str:>12} {current_str:>12} {delta_str:>10} {status_icon:>8}")

        print()
        print("=" * 90)

        # Detailed failures and warnings
        if warn_count > 0 or fail_count > 0:
            print()
            print("DETAILS:")
            print()

            for result in results:
                if result.status == 'fail':
                    print(f"✗ FAILURE: {result.metric_name}")
                    print(f"  Baseline: {result.baseline_value:.2f}")
                    print(f"  Current:  {result.current_value:.2f}")
                    print(f"  Delta:    {result.delta_percent:+.2f}%")
                    print(f"  Threshold: {result.threshold_fail:.2f}%")
                    print()

            for result in results:
                if result.status == 'warn':
                    print(f"⚠ WARNING: {result.metric_name}")
                    print(f"  Baseline: {result.baseline_value:.2f}")
                    print(f"  Current:  {result.current_value:.2f}")
                    print(f"  Delta:    {result.delta_percent:+.2f}%")
                    print(f"  Threshold: {result.threshold_warn:.2f}%")
                    print()

    def has_regressions(self, results: List[ComparisonResult]) -> bool:
        """Check if any regressions detected."""
        return any(r.status == 'fail' for r in results)

    def has_warnings(self, results: List[ComparisonResult]) -> bool:
        """Check if any warnings detected."""
        return any(r.status == 'warn' for r in results)


def main():
    parser = argparse.ArgumentParser(
        description="Compare FILE Relay Server load test baselines",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Compare two baselines
  %(prog)s baseline-v0.1.0.json baseline-v0.2.0.json

  # Fail CI build on regression
  %(prog)s baseline.json current.json --fail-on-regression

  # Fail on warnings too (strict mode)
  %(prog)s baseline.json current.json --fail-on-warning

  # Generate JSON output for automation
  %(prog)s baseline.json current.json --json > comparison.json
        """
    )

    parser.add_argument('baseline', help='Baseline results JSON file')
    parser.add_argument('current', help='Current results JSON file')
    parser.add_argument('--fail-on-regression', action='store_true',
                       help='Exit with code 1 if any regressions detected')
    parser.add_argument('--fail-on-warning', action='store_true',
                       help='Exit with code 1 if any warnings detected')
    parser.add_argument('--json', action='store_true',
                       help='Output results as JSON (for automation)')

    args = parser.parse_args()

    # Compare
    comparator = BaselineComparator(args.baseline, args.current)
    results = comparator.compare_all()

    # Output
    if args.json:
        output = {
            'baseline_file': args.baseline,
            'current_file': args.current,
            'results': [
                {
                    'metric': r.metric_name,
                    'baseline': r.baseline_value,
                    'current': r.current_value,
                    'delta': r.delta,
                    'delta_percent': r.delta_percent,
                    'status': r.status
                }
                for r in results
            ],
            'summary': {
                'pass': sum(1 for r in results if r.status == 'pass'),
                'warn': sum(1 for r in results if r.status == 'warn'),
                'fail': sum(1 for r in results if r.status == 'fail')
            }
        }
        print(json.dumps(output, indent=2))
    else:
        comparator.print_comparison(results)

    # Exit code
    exit_code = 0

    if args.fail_on_warning and comparator.has_warnings(results):
        print("Exiting with code 1 (warnings detected)", file=sys.stderr)
        exit_code = 1

    if args.fail_on_regression and comparator.has_regressions(results):
        print("Exiting with code 1 (regressions detected)", file=sys.stderr)
        exit_code = 1

    sys.exit(exit_code)


if __name__ == '__main__':
    main()
