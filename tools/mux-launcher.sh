#!/bin/bash
# MCP Server Launcher for rmcp-mux
# Created by M&K (c)2025 The LibraxisAI Team

CONFIG="$HOME/.rmcp_servers/config/mux.toml"
LOG_DIR="$HOME/.rmcp_servers/logs"
PID_DIR="$HOME/.rmcp_servers/pids"
SOCKET_DIR="$HOME/.rmcp_servers/sockets"

# Core servers to always start
CORE_SERVERS="loctree rmcp-memex brave-search"
# Utility servers (lazy start - still need daemon running)
UTIL_SERVERS="sequential-thinking youtube-transcript"

ALL_SERVERS="$CORE_SERVERS $UTIL_SERVERS"

mkdir -p "$LOG_DIR" "$PID_DIR" "$SOCKET_DIR"

start_server() {
    local name="$1"
    local pidfile="$PID_DIR/$name.pid"
    local logfile="$LOG_DIR/$name.log"

    if [ -f "$pidfile" ]; then
        local old_pid=$(cat "$pidfile")
        if kill -0 "$old_pid" 2>/dev/null; then
            echo "  [$name] Already running (PID $old_pid)"
            return 0
        fi
        rm -f "$pidfile"
    fi

    echo "  [$name] Starting..."
    nohup rmcp-mux --config "$CONFIG" --service "$name" > "$logfile" 2>&1 &
    local pid=$!
    echo "$pid" > "$pidfile"
    sleep 0.5

    if kill -0 "$pid" 2>/dev/null; then
        echo "  [$name] Started (PID $pid)"
    else
        echo "  [$name] FAILED - check $logfile"
        rm -f "$pidfile"
        return 1
    fi
}

stop_server() {
    local name="$1"
    local pidfile="$PID_DIR/$name.pid"

    if [ -f "$pidfile" ]; then
        local pid=$(cat "$pidfile")
        if kill -0 "$pid" 2>/dev/null; then
            echo "  [$name] Stopping (PID $pid)..."
            kill "$pid" 2>/dev/null
            sleep 0.5
            kill -9 "$pid" 2>/dev/null
        fi
        rm -f "$pidfile"
    fi
    rm -f "$SOCKET_DIR/$name.sock" 2>/dev/null
}

status_server() {
    local name="$1"
    local pidfile="$PID_DIR/$name.pid"
    local socket="$SOCKET_DIR/$name.sock"

    local status="STOPPED"
    local pid="-"

    if [ -f "$pidfile" ]; then
        pid=$(cat "$pidfile")
        if kill -0 "$pid" 2>/dev/null; then
            if [ -S "$socket" ]; then
                status="RUNNING"
            else
                status="STARTING"
            fi
        else
            status="DEAD"
            pid="-"
        fi
    fi

    printf "  %-20s %-10s %s\n" "$name" "$status" "$pid"
}

case "$1" in
    start)
        echo "Starting MCP servers..."
        for srv in $ALL_SERVERS; do
            start_server "$srv"
        done
        echo "Done."
        ;;
    stop)
        echo "Stopping MCP servers..."
        for srv in $ALL_SERVERS; do
            stop_server "$srv"
        done
        echo "Done."
        ;;
    restart)
        $0 stop
        sleep 1
        $0 start
        ;;
    status)
        echo "MCP Server Status:"
        printf "  %-20s %-10s %s\n" "SERVICE" "STATUS" "PID"
        echo "  ----------------------------------------"
        for srv in $ALL_SERVERS; do
            status_server "$srv"
        done
        echo ""
        echo "Sockets:"
        ls -la "$SOCKET_DIR"/*.sock 2>/dev/null || echo "  No sockets found"
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status}"
        exit 1
        ;;
esac
