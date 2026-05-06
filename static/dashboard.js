/* static/dashboard.js */

const MAX_POINTS = 300; // 5-minute window

let cpuChart, ramChart;

const cpuData = {
    labels: [],
    datasets: [{
        label: 'CPU %',
        data: [],
        borderColor: '#00f3ff',
        backgroundColor: 'rgba(0, 243, 255, 0.05)',
        borderWidth: 2,
        fill: true,
        pointRadius: 0,
        tension: 0.2
    }]
};

const ramData = {
    labels: [],
    datasets: [{
        label: 'RAM %',
        data: [],
        borderColor: '#bc13fe',
        backgroundColor: 'rgba(188, 19, 254, 0.05)',
        borderWidth: 2,
        fill: true,
        pointRadius: 0,
        tension: 0.2
    }]
};

function getChartColors() {
    const dark = document.documentElement.getAttribute('data-theme') === 'dark';
    return {
        cpu:  { line: dark ? '#00f3ff' : '#00b4d8', fill: dark ? 'rgba(0, 243, 255, 0.05)' : 'rgba(0, 180, 216, 0.05)' },
        ram:  { line: dark ? '#bc13fe' : '#5a189a', fill: dark ? 'rgba(188, 19, 254, 0.05)' : 'rgba(90, 24, 154, 0.05)' },
        grid: dark ? 'rgba(255,255,255,0.03)' : 'rgba(0,0,0,0.03)',
        tick: dark ? '#666666' : '#888888',
    };
}

function makeChartOptions(colors) {
    return {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        plugins: {
            legend: { display: false },
            tooltip: { 
                mode: 'index', 
                intersect: false,
                backgroundColor: '#000',
                titleFont: { family: 'Fira Code' },
                bodyFont: { family: 'Fira Code' },
                borderColor: '#222',
                borderWidth: 1
            }
        },
        scales: {
            y: {
                beginAtZero: true,
                max: 100,
                grid: { color: colors.grid },
                ticks: { color: colors.tick, font: { size: 10, family: 'Fira Code' }, callback: v => v + '%' }
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

// Add loading state to switches on click
document.addEventListener('click', function(e) {
    const switchEl = e.target.closest('.switch');
    if (switchEl && e.target.tagName === 'INPUT') {
        switchEl.classList.add('loading');
    }
});

document.body.addEventListener('htmx:afterSwap', function(evt) {
    if (evt.detail.target.id === 'dashboard') {
        updateCharts();
    }
});
