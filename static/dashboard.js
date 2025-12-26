/* static/dashboard.js */

const MAX_POINTS = 600; // 5 minutes window

let chart;
const chartData = {
    labels: [],
    datasets: [{
        label: 'CPU %',
        data: [],
        borderColor: '#2196F3',
        backgroundColor: 'rgba(33, 150, 243, 0.2)',
        borderWidth: 2,
        fill: true,
        pointRadius: 0,
        tension: 0.2
    }]
};

function initChart() {
    const ctx = document.getElementById('cpuChart');
    if (!ctx) return;

    if (chart) {
        chart.destroy();
    }

    chart = new Chart(ctx.getContext('2d'), {
        type: 'line',
        data: chartData,
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
}

initChart();

/**
 * Updates the chart with the current CPU value from the DOM
 */
function updateChart() {
    const cpuElement = document.getElementById('cpu-value');
    if (!cpuElement) return;

    const val = parseFloat(cpuElement.innerText);
    const time = new Date().toLocaleTimeString();

    chartData.labels.push(time);
    chartData.datasets[0].data.push(val);

    // Maintain sliding window
    if (chartData.labels.length > MAX_POINTS) {
        chartData.labels.shift();
        chartData.datasets[0].data.shift();
    }

    const ctx = document.getElementById('cpuChart');
    if (chart && chart.canvas === ctx) {
        chart.update('none');
    } else {
        initChart();
    }
}

// Listen for HTMX swaps to trigger chart update
document.body.addEventListener('htmx:afterSwap', function(evt) {
    if (evt.detail.target.id === 'dashboard') {
        updateChart();
    }
});