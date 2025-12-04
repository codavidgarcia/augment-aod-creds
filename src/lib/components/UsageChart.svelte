<script>
  import { onMount, onDestroy } from 'svelte';
  import { usageAnalytics } from '../stores/app.js';
  import Chart from 'chart.js/auto';
  import 'chartjs-adapter-date-fns';

  export let height = 300;
  export let type = 'balance'; // 'balance' or 'usage'

  let canvas;
  let chart;

  $: if (chart && $usageAnalytics) {
    updateChart($usageAnalytics);
  }

  function createChart(analytics) {
    if (!canvas || !analytics) return;

    const ctx = canvas.getContext('2d');
    
    // Destroy existing chart
    if (chart) {
      chart.destroy();
    }

    const isDark = document.documentElement.classList.contains('dark');
    const textColor = isDark ? '#e5e7eb' : '#374151';
    const gridColor = isDark ? '#374151' : '#e5e7eb';

    if (type === 'balance') {
      // Daily usage chart (shows credits consumed per day)
      const data = analytics.balance_history.map(point => ({
        x: new Date(point.timestamp),
        y: point.balance
      }));

      chart = new Chart(ctx, {
        type: 'bar',
        data: {
          datasets: [{
            label: 'Daily Usage',
            data: data,
            backgroundColor: 'rgba(59, 130, 246, 0.6)',
            borderColor: '#3b82f6',
            borderWidth: 1,
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: {
            intersect: false,
            mode: 'index'
          },
          plugins: {
            legend: {
              display: false
            },
            tooltip: {
              backgroundColor: isDark ? '#1f2937' : '#ffffff',
              titleColor: textColor,
              bodyColor: textColor,
              borderColor: gridColor,
              borderWidth: 1,
              callbacks: {
                label: function(context) {
                  return `Usage: ${context.parsed.y.toLocaleString()} credits`;
                }
              }
            }
          },
          scales: {
            x: {
              type: 'time',
              time: {
                unit: 'day',
                displayFormats: {
                  day: 'MMM d'
                }
              },
              grid: {
                color: gridColor,
                borderColor: gridColor
              },
              ticks: {
                color: textColor,
                maxTicksLimit: 8
              }
            },
            y: {
              beginAtZero: true,
              grid: {
                color: gridColor,
                borderColor: gridColor
              },
              ticks: {
                color: textColor,
                callback: function(value) {
                  return value.toLocaleString();
                }
              }
            }
          }
        }
      });
    } else {
      // Daily usage amounts chart (total credits per day)
      const data = analytics.usage_history.map(point => ({
        x: new Date(point.timestamp),
        y: point.usage_amount
      }));

      chart = new Chart(ctx, {
        type: 'line',
        data: {
          datasets: [{
            label: 'Credits Used',
            data: data,
            borderColor: '#ef4444',
            backgroundColor: 'rgba(239, 68, 68, 0.1)',
            borderWidth: 2,
            fill: true,
            tension: 0.4,
            pointRadius: 3,
            pointHoverRadius: 5,
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: {
            intersect: false,
            mode: 'index'
          },
          plugins: {
            legend: {
              display: false
            },
            tooltip: {
              backgroundColor: isDark ? '#1f2937' : '#ffffff',
              titleColor: textColor,
              bodyColor: textColor,
              borderColor: gridColor,
              borderWidth: 1,
              callbacks: {
                label: function(context) {
                  return `Usage: ${context.parsed.y.toLocaleString()} credits`;
                }
              }
            }
          },
          scales: {
            x: {
              type: 'time',
              time: {
                unit: 'day',
                displayFormats: {
                  day: 'MMM d'
                }
              },
              grid: {
                color: gridColor,
                borderColor: gridColor
              },
              ticks: {
                color: textColor,
                maxTicksLimit: 8
              }
            },
            y: {
              beginAtZero: true,
              grid: {
                color: gridColor,
                borderColor: gridColor
              },
              ticks: {
                color: textColor,
                callback: function(value) {
                  return value.toLocaleString();
                }
              }
            }
          }
        }
      });
    }
  }

  function updateChart(analytics) {
    if (!chart || !analytics) return;

    if (type === 'balance') {
      const data = analytics.balance_history.map(point => ({
        x: new Date(point.timestamp),
        y: point.balance
      }));
      chart.data.datasets[0].data = data;
    } else {
      const data = analytics.usage_history.map(point => ({
        x: new Date(point.timestamp),
        y: point.usage_amount
      }));
      chart.data.datasets[0].data = data;
    }

    chart.update('none');
  }

  onMount(() => {
    if ($usageAnalytics) {
      createChart($usageAnalytics);
    }

    // Listen for theme changes
    const observer = new MutationObserver(() => {
      if (chart && $usageAnalytics) {
        createChart($usageAnalytics);
      }
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class']
    });

    return () => observer.disconnect();
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
    }
  });
</script>

<div class="chart-container" style="height: {height}px;">
  {#if $usageAnalytics && ($usageAnalytics.balance_history.length > 0 || $usageAnalytics.usage_history.length > 0)}
    <canvas bind:this={canvas}></canvas>
  {:else}
    <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
      <div class="text-center">
        <div class="text-lg mb-2">📊</div>
        <div class="text-sm">No data available</div>
        <div class="text-xs">Charts will appear after collecting usage data</div>
      </div>
    </div>
  {/if}
</div>

<style>
  .chart-container {
    @apply relative w-full;
  }
  
  canvas {
    @apply w-full h-full;
  }
</style>
