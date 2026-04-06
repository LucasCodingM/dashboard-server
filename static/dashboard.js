/* static/dashboard.js */

const MAX_POINTS = 300; // 5-minute window

let cpuChart, ramChart;

const cpuData = {
    labels: [],
    datasets: [{
        label: 'CPU %',
        data: [],
        borderColor: '#60a5fa',
        backgroundColor: 'rgba(96,165,250,0.12)',
        borderWidth: 2,
        fill: true,
        pointRadius: 0,
        tension: 0.3
    }]
};

const ramData = {
    labels: [],
    datasets: [{
        label: 'RAM %',
        data: [],
        borderColor: '#a78bfa',
        backgroundColor: 'rgba(167,139,250,0.12)',
        borderWidth: 2,
        fill: true,
        pointRadius: 0,
        tension: 0.3
    }]
};

function getChartColors() {
    const dark = document.documentElement.getAttribute('data-theme') === 'dark';
    return {
        cpu:  { line: dark ? '#60a5fa' : '#3b82f6', fill: dark ? 'rgba(96,165,250,0.12)'  : 'rgba(59,130,246,0.12)'  },
        ram:  { line: dark ? '#a78bfa' : '#8b5cf6', fill: dark ? 'rgba(167,139,250,0.12)' : 'rgba(139,92,246,0.12)'  },
        grid: dark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)',
        tick: dark ? '#94a3b8' : '#9ca3af',
    };
}

function makeChartOptions(colors) {
    return {
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
                ticks: { color: colors.tick, font: { size: 10 }, callback: v => v + '%' }
            },
            x: {
                grid: { display: false },
                ticks: { display: false }
            }
        }
    };
}

function initCharts() {
    const colors = getChartColors();

    const cpuCtx = document.getElementById('cpuChart');
    if (cpuCtx) {
        if (cpuChart) cpuChart.destroy();
        cpuData.datasets[0].borderColor = colors.cpu.line;
        cpuData.datasets[0].backgroundColor = colors.cpu.fill;
        cpuChart = new Chart(cpuCtx.getContext('2d'), {
            type: 'line',
            data: cpuData,
            options: makeChartOptions(colors)
        });
    }

    const ramCtx = document.getElementById('ramChart');
    if (ramCtx) {
        if (ramChart) ramChart.destroy();
        ramData.datasets[0].borderColor = colors.ram.line;
        ramData.datasets[0].backgroundColor = colors.ram.fill;
        ramChart = new Chart(ramCtx.getContext('2d'), {
            type: 'line',
            data: ramData,
            options: makeChartOptions(colors)
        });
    }
}

function updateChartTheme(theme) {
    const colors = getChartColors();

    if (cpuChart) {
        cpuChart.data.datasets[0].borderColor = colors.cpu.line;
        cpuChart.data.datasets[0].backgroundColor = colors.cpu.fill;
        cpuChart.options.scales.y.grid.color = colors.grid;
        cpuChart.options.scales.y.ticks.color = colors.tick;
        cpuChart.update('none');
    }

    if (ramChart) {
        ramChart.data.datasets[0].borderColor = colors.ram.line;
        ramChart.data.datasets[0].backgroundColor = colors.ram.fill;
        ramChart.options.scales.y.grid.color = colors.grid;
        ramChart.options.scales.y.ticks.color = colors.tick;
        ramChart.update('none');
    }
}

initCharts();

function pushPoint(chartDataset, value) {
    chartDataset.labels.push('');
    chartDataset.datasets[0].data.push(value);
    if (chartDataset.labels.length > MAX_POINTS) {
        chartDataset.labels.shift();
        chartDataset.datasets[0].data.shift();
    }
}

function updateCharts() {
    const cpuEl = document.getElementById('cpu-value');
    const ramEl = document.getElementById('ram-value');

    if (cpuEl) pushPoint(cpuData, parseFloat(cpuEl.innerText));
    if (ramEl) pushPoint(ramData, parseFloat(ramEl.innerText));

    // Reuse existing chart instances if canvases are preserved
    const cpuCtx = document.getElementById('cpuChart');
    const ramCtx = document.getElementById('ramChart');

    if (cpuChart && cpuCtx && cpuChart.canvas === cpuCtx) {
        cpuChart.update('none');
    } else {
        initCharts();
        return;
    }

    if (ramChart && ramCtx && ramChart.canvas === ramCtx) {
        ramChart.update('none');
    } else {
        initCharts();
    }
}

document.body.addEventListener('htmx:afterSwap', function(evt) {
    if (evt.detail.target.id === 'dashboard') {
        updateCharts();
    }
});
