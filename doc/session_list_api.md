# Session List WebSocket API

Messages are a newline-delimited list of parameters, with the first parameter being a command name.

## Client → Server

### Connect

```
connect
[user_id: String]
[username: String]
[version: String]
```

Used for usage metrics.

## Server → Client

### New Connection

```
reset
[current_connection_id: i32]
[expected_client_version: String]
```

Send on new connections. Instructs the client to remove all sessions that do NOT have the provided connection id. Additionally, it contains an expected version number so that the client can display a warning if outdated. After this message is sent, the server will send a new Add Session command for each currently known session.

### Remove Session

```
reset
[current_connection_id: i32]
[session_id: string]
```

Instructs the client to remove a session.

### Add Session

```
add
[current_connection_id: i32]
[session_id: string]
[active_users: i32]
[joined_users: i32]
[host_username: String]
[host_registration_date: DateTime<Utc>]
[host_is_patron: bool]
[host_is_mentor: bool]
[session_name: String]
[session_begin_time: DateTime<Utc>]
[users: USER_ARRAY]
```

The user array is a list of users. As it cannot be newline-delimited, it is instead carriage-return delimited. For convenience, it is shown below newline-delimited.
```
[user_id: String]
[username: String]
[registration_date: DateTime<Utc>]
[is_patron: bool]
[is_mentor: bool]
```

Instructs the client to add a session.

### Update Session
```
update
[current_connection_id: i32]
[session_id: String]
[session_name: String]
[active_users: i32]
[joined_users: i32]
```

Instructs the client to update mutable details about a session.

### Add User

```
[current_connection_id: i32]
[session_id: String]
[user_id: String]
[username: String]
[registration_date: DateTime<Utc>]
[is_patron: bool]
[is_mentor: bool]
```

Instructs the client to add a user to a session.

### Remove User

```
[current_connection_id: i32]
[session_id: String]
[user_id: String]
[username: String]
```

Instructs the client to remove a user from a session.

### Error
```
[current_connection_id: i32]
[error: String]
```

Instructs the client to log an error.
