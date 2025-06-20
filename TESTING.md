# Testing the OPC UA Client

## Test Servers

To test this client, you'll need an OPC UA server running. Here are some options:

### 1. Free OPC UA Servers

**Online Test Servers:**

- `opc.tcp://opcuaserver.com:48010` - Free online test server
- `opc.tcp://milo.digitalpetri.com:62541/milo` - Eclipse Milo test server

**Local Test Servers:**

- Prosys OPC UA Simulation Server (free version available)
- UAExpert OPC UA Client (includes demo server)
- KEPServerEX (trial version)

### 2. Testing Steps

1. **Start your OPC UA server**
2. **Run the client:**
   ```bash
   cargo run
   ```
3. **Connect to server:**
    - Press `C` or `Alt+F` to open connection dialog
    - Enter server URL (e.g., `opc.tcp://localhost:4840`)
    - Press Enter to connect

4. **Browse nodes:**
    - Press `1` to enter browse mode
    - Use arrow keys to navigate
    - Press Enter on folders to expand them
    - Common starting points:
        - Objects → Server → ServerStatus
        - Objects → Demo → SimulationMass

5. **Add subscriptions:**
    - Navigate to a variable node
    - Press `A` to add to subscription
    - Press `2` to view active subscriptions

6. **Read properties:**
    - Select any node in browse mode
    - Press `P` to view node properties

7. **Write values:**
    - Select a writable variable node
    - Press `W` to write a new value
    - Enter the value and press Enter

### 3. Common Node IDs

Some standard OPC UA nodes you can try browsing to:

- **Root folder:** `ns=0;i=84`
- **Objects folder:** `ns=0;i=85`
- **Types folder:** `ns=0;i=86`
- **Views folder:** `ns=0;i=87`
- **Server object:** `ns=0;i=2253`
- **Server status:** `ns=0;i=2256`

### 4. Troubleshooting

**Connection Issues:**

- Check if server is running
- Verify correct port number
- Try "None" security policy first
- Check firewall settings

**Browse Issues:**

- Some servers require authentication
- Check node access permissions
- Verify node IDs are correct

**Subscription Issues:**

- Only Variable nodes can be subscribed
- Check if node supports monitoring
- Verify subscription was created successfully

### 5. Example Session

```
1. Start client: cargo run
2. Press Alt+F (or C) to connect
3. Enter: opc.tcp://opcuaserver.com:48010
4. Press Enter to connect
5. Press 1 to browse
6. Navigate to Demo folder
7. Select a variable node
8. Press A to add to subscription
9. Press 2 to view live values
```
