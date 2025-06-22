# Enhanced Connection Logging Guide

The OPC UA client now includes comprehensive console logging to help you understand what's happening during the
connection process.

## What's Been Added

### 🔄 Connection Process Logging

- **Connection initiation**: Shows target endpoint and timeout settings
- **Client configuration**: Details about the OPC UA client setup (app name, URI, security settings)
- **Endpoint creation**: Shows security mode, policy, and authentication method
- **TCP handshake**: Indicates when the actual network connection is being established
- **Session establishment**: Confirms successful OPC UA session creation
- **Troubleshooting tips**: Provides helpful hints if connection hangs

### 🔍 Browse Operation Logging

- **Browse requests**: Shows which node is being browsed
- **Server responses**: Details about references found and their types
- **Node information**: Displays node names, classes, and whether they have children
- **Error handling**: Clear indication when browse operations fail and fallback to demo data

### 📊 Attribute Reading Logging

- **Attribute requests**: Shows which attributes are being read from nodes
- **Read results**: Details about successful attribute reads with values and types
- **Status reporting**: Indicates which attributes were skipped due to errors
- **Data types**: Shows the OPC UA data type for each successfully read attribute

### 🔌 Disconnection Logging

- **Disconnect process**: Shows when disconnection is initiated
- **Resource cleanup**: Confirms client resources are properly cleaned up
- **Status updates**: Shows connection status changes

## How to See the Logging

### Method 1: Run from PowerShell/Command Prompt

1. Open PowerShell or Command Prompt
2. Navigate to the project directory:
   ```powershell
   cd "d:\Source Code\jkopcclient\jk-opc-client"
   ```
3. Run the application:
   ```powershell
   cargo run
   ```
4. The console logging will appear in the same terminal window alongside the TUI

### Method 2: Run from VS Code Terminal

1. Open VS Code with your project
2. Open a new terminal in VS Code (Ctrl+`)
3. Run: `cargo run`
4. The logging will appear in the VS Code terminal

## Example Log Output

When you connect to an OPC UA server, you'll see output like this:

```
🔄 Starting OPC UA connection process...
📍 Target endpoint: opc.tcp://localhost:4840
⏱️  Connection timeout: 30 seconds
📡 Creating OPC UA client configuration...
🔧 Building client with the following configuration:
   📱 Application Name: 'OPC UA TUI Client'
   🔗 Application URI: 'urn:opcua-tui-client'
   🔐 Trust Server Certs: true
   🔄 Session Retry Limit: 3
   🗝️  Creating sample keypair: true
🏗️  Creating client instance...
✅ Client instance created successfully
🔗 Creating endpoint description:
   🌐 URL: opc.tcp://localhost:4840
   🔒 Security Mode: None (no encryption)
   📜 Security Policy: None
   🎫 Authentication: Anonymous
🚀 Attempting to connect to endpoint...
⏳ Establishing TCP connection and performing OPC UA handshake...
💡 If this hangs, check:
   • Server is running and accessible
   • Port is not blocked by firewall
   • URL is correct (e.g., opc.tcp://localhost:4840)
✅ Successfully established session with OPC UA server
🎉 Connection completed - ready for browsing and reading!
💾 Storing session and client references
🎊 Connection process completed successfully!
🔗 Status: Connected to opc.tcp://localhost:4840
```

When browsing nodes:

```
🔍 Browsing OPC UA node: i=84
📖 Using active session to browse node
🔧 Creating browse description:
   🎯 Node ID: i=84
   ➡️  Browse Direction: Forward
   🔗 Reference Type: HierarchicalReferences
   📊 Include Subtypes: true
🚀 Executing browse request to OPC UA server...
✅ Browse request successful
📋 Found 3 references:
   📄 Node: Objects | Class: Object | Children: true
   📄 Node: Types | Class: ObjectType | Children: true
   📄 Node: Views | Class: Object | Children: true
📦 Returning 3 nodes from browse operation
```

## Troubleshooting with Logs

The enhanced logging helps you troubleshoot common issues:

### Connection Issues

- **"Establishing TCP connection and performing OPC UA handshake..."** hangs: Server may be down or unreachable
- **"Failed to create client"**: Client configuration issue
- **Connection errors**: Check the specific error message for details

### Browse Issues

- **"Browse failed"**: The server may not support browsing or the node doesn't exist
- **"Falling back to demo data"**: Real browsing failed, using demo data instead
- **"No references found"**: The node exists but has no children

### Attribute Reading Issues

- **"Failed to read attribute"**: The attribute may not exist or be accessible
- **"Skipping attribute"**: Attribute read returned an error status
- **"No active session"**: Connection was lost, returning demo data

## Tips for Effective Debugging

1. **Watch the connection sequence**: Each step should complete before the next begins
2. **Check timing**: If there are long pauses, it may indicate network issues
3. **Monitor error patterns**: Consistent failures may indicate configuration issues
4. **Use the logging for support**: Include relevant log output when reporting issues

The logging is designed to be informative without being overwhelming, using emojis and clear descriptions to make it
easy to follow the application's behavior.
