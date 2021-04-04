# Unstable API Documentation
These APIs are still a work in progress. They **will** be changed without
notice. This documentation is intended not for API users, but for me to
keep track of the current API for my testing.

## Timestamp Storage
```
POST /initTime      "100" => 200 OK with body "100"
POST /initTimeForce "100" => 200 OK with body "100"
POST /initTimeReset       => 200 OK
GET  /initTimePeek        => 200 OK with body "Some(100)"
```

## Counter
```
GET /counter => 200 OK with body "Some(0)"
```

## System Statistics
```
GET /systemstat => 200 OK with body containing many system stats
```
