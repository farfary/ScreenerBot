# CLI Arguments Reference

## Display and Demo Mode

### --dashboard-demo

Enable hardcoded dashboard demo data for screenshots, marketing, and UI review.

**Usage**:

```bash
screenerbot --dashboard-demo
screenerbot --gui --dashboard-demo
```

**Notes**:

- Demo mode affects dashboard API responses that have demo fixtures.
- In GUI mode, Electron still passes `--gui`; demo mode is an additional backend flag.
- The wrapper launcher supports this through `./run.sh demo` or `./run.sh headless --demo`.

## Webserver Configuration

### --port <PORT>

Override the webserver port (1-65535, default: 8080).

**Usage**:

```bash
screenerbot --port 9000
```

**Notes**:

- Invalid values cause bot to exit with error
- GUI mode ignores this and uses dynamic port
- Ports below 1024 require elevated privileges

**Examples**:

```bash
screenerbot --port 3000     # Custom port
sudo screenerbot --port 80  # Privileged port
```

### --host <HOST>

Override the webserver host (default: 127.0.0.1).

**Usage**:

```bash
screenerbot --host 0.0.0.0
```

**Notes**:

- IPv4 addresses only
- Use 0.0.0.0 for remote access (security risk)
- Empty values cause bot to exit with error

**Examples**:

```bash
screenerbot --host 127.0.0.1       # Localhost only (default)
screenerbot --host 0.0.0.0         # All interfaces (remote access)
screenerbot --host 192.168.1.100  # Specific interface
```

### Combined Usage

```bash
screenerbot --port 3000 --host 0.0.0.0
```

Precedence: CLI arguments > config file > defaults

## Troubleshooting

### Port Already in Use

Error: `Address already in use`

**Solution**:

1. Check what's using the port: `lsof -i :8080`
2. Kill the process or use a different port
3. Use `--port` to specify alternative port

### Permission Denied

Error: `Permission denied`

**Cause**: Port below 1024 requires elevated privileges

**Solution**:

- Linux/macOS: Use `sudo screenerbot --port 80`
- Windows: Run as Administrator

### Invalid Port Value

Error: `Invalid port value 'abc'`

**Cause**: Port must be a number between 1-65535

**Solution**: Use valid numeric port value

### Empty Host

Error: `Host cannot be empty`

**Cause**: --host provided without value

**Solution**: Provide valid host: `--host 0.0.0.0` or `--host 127.0.0.1`
