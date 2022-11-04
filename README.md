<img src="assets/logo.svg" width=400>

Concept
-------

Servio uses layered architecture for its work. It consists of three layers:

<img src="assets/components.svg">

**Application** - endpoint layer, implementing project business-logic.  
**Middleware** - transformation layer, that passes events between Server and Applications and vice versa.  
**Server** - transport layer, that handles network requests and transforms them into stream of outgoung events.

Servio decomposes network protocols into a pair of event streams, providing bidirectional async communication between 
a server and an application.

Servio was carefully modeled after [ASGI](https://asgi.readthedocs.io/), the asynchronous web interface in Python and uses its core concepts.

Component stability
-------------------

| Component      | Stability    |
|----------------|--------------|
| servio-service | stable       |
| servio-http    | stable       |
| servio-hyper   | unstable     |
| servio-util    | experimental |
