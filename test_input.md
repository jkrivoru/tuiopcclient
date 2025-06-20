# Testing Input Issues

## Issues to Test:

1. **Double Input Issue**:
    - Type a character and see if it appears once or twice
    - Test with letters, numbers, and special characters
    - Test backspace and arrow keys

2. **Cursor Visibility**:
    - Check if cursor is visible in server URL field ✅ (working)
    - Check if cursor is visible in username field (need to test)
    - Check if cursor is visible in password field (need to test)

3. **Field Navigation**:
    - Test Tab key to navigate between fields
    - Test arrow keys for cursor movement within fields
    - Test switching between username and password fields

## Test Steps:

1. Start application
2. Type in server URL field - check for double characters
3. Press Enter to discover endpoints
4. Select an endpoint
5. Switch to Username/Password authentication
6. Test Tab key to switch between username and password
7. Test typing in both fields
8. Test cursor visibility in both fields

## Expected Behavior:

- Single character per keypress
- Visible cursor in all input fields
- Smooth field navigation
- Proper input handling for masked password field
