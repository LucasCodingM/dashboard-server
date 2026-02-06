# Dashboard Server

A lightweight, self-hosted dashboard written in Rust to monitor server statistics, manage services, and handle downloads. Built with [Axum](https://github.com/tokio-rs/axum), [Askama](https://github.com/djc/askama), and [HTMX](https://htmx.org/).

## Features

- **System Monitoring**: Real-time tracking of CPU usage, CPU temperature, RAM usage, and Disk space.
- **Power Consumption**: Real-time power usage estimation using Intel RAPL (Running Average Power Limit).
- **Service Management**: View status and Start/Stop specific services (e.g., Samba, MiniDLNA, custom bots).
- **Download Manager**:
  - Downloads videos using `yt-dlp` (automatically detects YouTube/video URLs).
  - Downloads generic files using `wget`.
  - Categorizes downloads into Movies, Videos, or general Downloads.
- **System Control**: Remote Shutdown and Reboot.
- **Authentication**: Basic session-based access control.

## Prerequisites

- **Rust**: Stable toolchain.
- **OS**: Linux (relies on `systemd`, `/sys` filesystem, and standard Linux commands).
- **External Tools**:
  - `yt-dlp`: For video downloading.
  - `wget`: For file downloading.
- **Permissions**: The user running the application requires `sudo` privileges for specific commands.

## Configuration

### Environment Variables

Create a `.env` file in the project root to configure download destinations:

```env
MOVIE_PATH=/stockage/videos/films
VIDEO_PATH=/stockage/videos
DOWNLOAD_PATH=/stockage/telechargements
```

### Sudoers Setup

Since the application executes system commands (`systemctl`, `shutdown`, `reboot`), you should configure `sudo` to allow these without a password for the user running the dashboard.

Add the following to your `/etc/sudoers` (using `visudo`):

```bash
# Replace 'your_user' with the user running the dashboard
your_user ALL=(ALL) NOPASSWD: /usr/bin/systemctl start *, /usr/bin/systemctl stop *, /sbin/shutdown, /sbin/reboot
```

*Note: Reading RAPL power stats usually requires root privileges or specific read permissions on `/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj`.*

## Installation & Development

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd dashboard-server
   ```

2. **Build and Run**:
   ```bash
   cargo run --release
   ```
   The server will start at `http://localhost:3000`.

## Deployment

A `deploy.sh` script is provided to automate deployment to a remote server.

1. **Configure Deployment**:
   Edit `deploy.sh` and update the variables:
   ```bash
   SERVER="user@your-server.local"
   DEST="/home/lucas/app/dashboard"
   ```

2. **Deploy**:
   ```bash
   ./deploy.sh
   ```
   This script will:
   - Compile the binary in release mode.
   - Rsync the binary, templates, static assets, and `.env` file to the server.
   - Restart the systemd service (assumes a service named `dashboard` exists).