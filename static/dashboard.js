/* static/dashboard.js */

const MAX_POINTS = 600; // 5-minute window

let chart;

const chartData = {
    labels: [],
    datasets: [{
        label: 'CPU %',
        data: [],
        borderColor: '#60a5fa',
        backgroundColor: 'rgba(96, 165, 250, 0.15)',
        borderWidth: 2,
        fill: true,
        pointRadius: 0,
        tension: 0.3
    }]
};

function getChartColors() {
    const dark = document.documentElement.getAttribute('data-theme') === 'dark';
    return {
        line:  dark ? '#60a5fa' : '#3b82f6',
        fill:  dark ? 'rgba(96,165,250,0.12)' : 'rgba(59,130,246,0.12)',
        grid:  dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)',
        tick:  dark ? '#94a3b8' : '#6b7280',
    };
}

function initChart() {
    const ctx = document.getElementById('cpuChart');
    if (!ctx) return;
    if (chart) chart.destroy();

    const colors = getChartColors();
    chartData.datasets[0].borderColor = colors.line;
    chartData.datasets[0].backgroundColor = colors.fill;

    chart = new Chart(ctx.getContext('2d'), {
        type: 'line',
        data: chartData,
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            plugins: {
                legend: { display: false },
                tooltip: { mode: 'index', intersect: false }
            },
            scales: {
                y: {
                    beginAtZero: true,
                    max: 100,
                    grid: { color: colors.grid },
                    ticks: { color: colors.tick, font: { size: 11 } }
                },
                x: {
                    grid: { display: false },
                    ticks: {
                        display: true,
                        autoSkip: true,
                        maxTicksLimit: 8,
                        color: colors.tick,
                        font: { size: 11 }
                    }
                }
            }
        }
    });
}

function updateChartTheme(theme) {
    if (!chart) return;
    const colors = getChartColors();
    chart.data.datasets[0].borderColor = colors.line;
    chart.data.datasets[0].backgroundColor = colors.fill;
    chart.options.scales.y.grid.color = colors.grid;
    chart.options.scales.y.ticks.color = colors.tick;
    chart.options.scales.x.ticks.color = colors.tick;
    chart.update('none');
}

initChart();

function updateChart() {
    const cpuElement = document.getElementById('cpu-value');
    if (!cpuElement) return;

    const val = parseFloat(cpuElement.innerText);
    const time = new Date().toLocaleTimeString();

    chartData.labels.push(time);
    chartData.datasets[0].data.push(val);

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

document.body.addEventListener('htmx:afterSwap', function(evt) {
    if (evt.detail.target.id === 'dashboard') {
        updateChart();
    }
});
