# Troubleshooting

If a resource cannot be read, first confirm whether it came from the active Surface lists or from Catalog and Details. For BrokerOnly Resources, use the typed static ResourceLink returned by Details or the concrete URI expanded from an approved template.

Do not treat the standard lists as a BrokerOnly total directory. Refresh catalog information or review the relevant capability details before retrying a standard read.

`resources/subscribe` and `resources/unsubscribe` are outside the current Unify broker contract for built-in guides and BrokerOnly routes. If a client automatically attempts a subscription and receives an error, evaluate it separately from `resources/read`; continue using the standard read URI to retrieve content.
