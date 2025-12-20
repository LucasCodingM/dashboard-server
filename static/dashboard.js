/* static/dashboard.js */

const MAX_POINTS = 600; // 5 minutes window
const ctx = document.getElementById('cpuChart').getContext('2d');

const chart = new Chart(ctx, {
    type: 'line',
    data: {
        labels: [],
        datasets: [{
            label: 'CPU Usage %',
            data: [],
            borderColor: '#4caf50',
            backgroundColor: 'rgba(76, 175, 80, 0.2)',
            borderWidth: 2,
            fill: true,
            pointRadius: 0,
            tension: 0.2
        }]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        scales: {
            y: { beginAtZero: true, max: 100 },
            x: {
                ticks: { display: true, autoSkip: true, maxTicksLimit: 10 }
            }
        }
    }
});

/**
 * Updates the chart with the current CPU value from the DOM
 */
function updateChart() {
    const cpuElement = document.getElementById('cpu-value');
    if (!cpuElement) return;

    const val = parseFloat(cpuElement.innerText);
    const time = new Date().toLocaleTimeString();

    chart.data.labels.push(time);
    chart.data.datasets[0].data.push(val);

    // Maintain sliding window
    if (chart.data.labels.length > MAX_POINTS) {
        chart.data.labels.shift();
        chart.data.datasets[0].data.shift();
    }

    chart.update('none'); // Update without animation for performance
}

// Listen for HTMX swaps to trigger chart update
document.body.addEventListener('htmx:afterSwap', function(evt) {
    if (evt.detail.target.id === 'dashboard') {
        updateChart();
    }
});