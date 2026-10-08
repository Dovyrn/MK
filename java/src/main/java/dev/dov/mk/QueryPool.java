package dev.dov.mk;

public record QueryPool(long raw) {
    static QueryPool of(long raw) {
        return raw == 0 ? null : new QueryPool(raw);
    }

    static long raw(QueryPool handle) {
        return handle == null ? 0 : handle.raw;
    }

    public Resource resource() {
        return new Resource().kind(ResourceKind.QUERY_POOL).handle(raw);
    }
}
