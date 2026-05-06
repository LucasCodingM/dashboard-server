/* static/dashboard.js */

const MAX_POINTS = 300; // 5-minute window

let cpuChart, ramChart, netChart;

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

const netData = {
    labels: [],
    datasets: [
        {
            label: 'RX (Down)',
            data: [],
            borderColor: '#39ff14',
            backgroundColor: 'rgba(57, 255, 20, 0.05)',
            borderWidth: 1.5,
            fill: true,
            pointRadius: 0,
            tension: 0.3
        },
        {
            label: 'TX (Up)',
            data: [],
            borderColor: '#00f3ff',
            backgroundColor: 'rgba(0, 243, 255, 0.05)',
            borderWidth: 1.5,
            fill: true,
            pointRadius: 0,
            tension: 0.3
        }
    ]
};

function getChartColors() {
    const dark = document.documentElement.getAttribute('data-theme') === 'dark';
    return {
        cpu:  { line: dark ? '#00f3ff' : '#00b4d8', fill: dark ? 'rgba(0, 243, 255, 0.05)' : 'rgba(0, 180, 216, 0.05)' },
        ram:  { line: dark ? '#bc13fe' : '#5a189a', fill: dark ? 'rgba(188, 19, 254, 0.05)' : 'rgba(90, 24, 154, 0.05)' },
        net_rx: { line: '#39ff14', fill: 'rgba(57, 255, 20, 0.05)' },
        net_tx: { line: '#00f3ff', fill: 'rgba(0, 243, 255, 0.05)' },
        grid: dark ? 'rgba(255,255,255,0.03)' : 'rgba(0,0,0,0.03)',
        tick: dark ? '#666666' : '#888888',
    };
}

function makeChartOptions(colors, isNet = false) {
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
                max: isNet ? undefined : 100,
                grid: { color: colors.grid },
                ticks: { 
                    color: colors.tick, 
                    font: { size: 10, family: 'Fira Code' }, 
                    callback: v => isNet ? formatBytes(v) : v + '%' 
                }
            },
            x: {
                grid: { display: false },
                ticks: { display: false }
            }
        }
    };
}

function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
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

    const netCtx = document.getElementById('netChart');
    if (netCtx) {
        if (netChart) netChart.destroy();
        netChart = new Chart(netCtx.getContext('2d'), {
            type: 'line',
            data: netData,
            options: makeChartOptions(colors, true)
        });
    }
}

function updateChartTheme(theme) {
    const colors = getChartColors();
    [cpuChart, ramChart].forEach(chart => {
        if (chart) {
            chart.options.scales.y.grid.color = colors.grid;
            chart.options.scales.y.ticks.color = colors.tick;
            chart.update('none');
        }
    });
    if (netChart) {
        netChart.options.scales.y.grid.color = colors.grid;
        netChart.options.scales.y.ticks.color = colors.tick;
        netChart.update('none');
    }
}

initCharts();

function pushPoint(chartDataset, value, index = 0) {
    if (index === 0) chartDataset.labels.push('');
    chartDataset.datasets[index].data.push(value);
    if (chartDataset.labels.length > MAX_POINTS) {
        if (index === 0) chartDataset.labels.shift();
        chartDataset.datasets[index].data.shift();
    }
}

function updateCharts() {
    const cpuEl = document.getElementById('cpu-value');
    const ramEl = document.getElementById('ram-value');
    const rxEl = document.getElementById('rx-raw');
    const txEl = document.getElementById('tx-raw');

    if (cpuEl) pushPoint(cpuData, parseFloat(cpuEl.innerText));
    if (ramEl) pushPoint(ramData, parseFloat(ramEl.innerText));
    if (rxEl && txEl) {
        pushPoint(netData, parseFloat(rxEl.innerText), 0);
        pushPoint(netData, parseFloat(txEl.innerText), 1);
    }

    [cpuChart, ramChart, netChart].forEach(chart => {
        if (chart) chart.update('none');
    });
}

// Add loading state to switches on click
document.addEventListener('click', function(e) {
    const switchEl = e.target.closest('.switch');
    if (switchEl && e.target.tagName === 'INPUT') {
        switchEl.classList.add('loading');
    }
});

let currentSort = { key: 'name', asc: true };

function sortContainers(key) {
    if (currentSort.key === key) {
        currentSort.asc = !currentSort.asc;
    } else {
        currentSort.key = key;
        currentSort.asc = true;
    }
    applySort();
}

function parseVal(val, key) {
    if (key === 'status') return parseInt(val);
    if (key === 'name') return val.toLowerCase();
    // For CPU, MEM, NET - extract numbers
    const match = val.match(/([0-9.]+)/);
    return match ? parseFloat(match[1]) : 0;
}

function applySort() {
    const list = document.getElementById('container-list');
    if (!list) return;
    const items = Array.from(list.children);
    
    items.sort((a, b) => {
        const valA = parseVal(a.getAttribute(`data-${currentSort.key}`), currentSort.key);
        const valB = parseVal(b.getAttribute(`data-${currentSort.key}`), currentSort.key);
        
        if (valA < valB) return currentSort.asc ? -1 : 1;
        if (valA > valB) return currentSort.asc ? 1 : -1;
        return 0;
    });

    items.forEach(item => list.appendChild(item));
}

document.body.addEventListener('htmx:afterSwap', function(evt) {
    if (evt.detail.target.id === 'dashboard') {
        updateCharts();
        applySort(); // Re-apply sort after HTMX update
    }
});
