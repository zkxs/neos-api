# API
These APIs are subject to change as this project is still very much a work in progress.

## New User Session List

Returns a list of sessions that are likely related to new users. Mentors rejoice!

The first character of the response will be `N` if there is a new element not present the last time this route was called, otherwise it will be `X`. This feature is intended to be used to play a notification sound when a new session appears.

**Request:** `GET http://localhost:3030/sessionlist`

**Example Response:**
```
NPoxAzraelis (The Avatar Station) (1/1) 1:35 2021-04-03
huskyeet (Neos Hub) (1/1) 3:39 2021-04-03
danny_gryphon (Metaverse Training Center) (1/1) 5:18 2021-03-20
Unun (MTC Avatar Lobby) (1/1) 21:35 2021-04-03
```

Fields, in order of appearance:

1. Host username
2. World name
3. Active users in session / Total users in session
4. Session uptime
5. Host user registration date
6. The word "patron" if the host is a patron, otherwise absent

## Session List WebSocket
Streams session list updates. See the
[protocol documentation](session_list_api.md) for details.

## Global Public User List

Outputs a newline-delimited list of all users publicly visible as online. This requires them to be in a public session. I am not sure if the Invisible status hides users. Headless users are not filtered out.

Unregistered users will have a `?` prefix prepended to their username. If there are multiple unregistered users with the same name they will be combined as there is no way to distinguish between them.

**Request:** `GET http://localhost:3030/users`

**Example Response:**
```
3x1t_5tyl3
?Nizo DBF
runtime
```

This is a contrived example. Typical outputs will have upwards of 70 lines.

## User Registration Date
Looks up the IS0-8601 formatted registration date of a user.

**Request:** `GET http://localhost:3030/userRegistration/[user_id]`

**Example Request:** `GET http://localhost:3030/userRegistration/U-runtime`

**Example Response:**
```
2020-10-13T19:41:20Z
```

## HTTP Test
Takes a string path parameter and sends it back to you. Useless, aside from testing Logix.

**Request:** `GET http://localhost:3030/hello/[string]`

**Example Request:** `GET http://localhost:3030/hello/foo`

**Example Response:**
```
Hello, foo!
```

## WebSocket Test

Two different debug websockets. Useless, aside from testing logix.

### Echo

**URL:** `ws://localhost:3030/echo`

**Example Request:**
```
foo
```

**Example Response:**
```
foo
```

### Mutate

**URL:** `ws://localhost:3030/wshello`

**Example Request:**
```
foo
```

**Example Response:**
```
Hello, foo!
```
