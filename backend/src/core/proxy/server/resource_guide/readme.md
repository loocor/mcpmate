# MCPMate Resource Readme

Use the standard Resource endpoints to read Resource URIs for the current consumer.

`resources/list` and `resources/templates/list` describe the active Surface. They are not the BrokerOnly total directory.

Active Surface URIs come from standard lists. BrokerOnly URIs come from Catalog/Details. Both a typed static ResourceLink and an expanded template concrete URI are read with standard `resources/read`.

For a resource workflow, move from Catalog to Details and then use standard `resources/read`. The catalog identifies the available capability, details explain its route and input, and the standard read returns the resource content.

The current Unify broker contract covers Resource discovery and standard reads. It does not provide `resources/subscribe` or `resources/unsubscribe` for MCPMate built-in guides or BrokerOnly upstream routes. A client may still attempt subscription because MCPMate exposes subscription support for other Resource surfaces; a rejected subscription does not mean `resources/read` failed.

The stable guide topics are overview, catalog, details, read, and troubleshooting. The guide template URI is `mcpmate://resources/template/mcpmate/guide/{topic}`. Preserve the `/template/` segment when constructing a guide URI.
