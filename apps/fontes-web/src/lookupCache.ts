// Session-only cache for immutable study data. Callers include snapshot and edition in keys.
export function createLookupCache(limit = 16) {
  const entries = new Map<string, Promise<unknown>>();
  return {
    clear() {
      entries.clear();
    },
    load<T>(key: string, fetch: () => Promise<T>): Promise<T> {
      const existing = entries.get(key);
      if (existing) {
        entries.delete(key);
        entries.set(key, existing);
        return existing as Promise<T>;
      }
      const pending = fetch().catch((error) => {
        if (entries.get(key) === pending) entries.delete(key);
        throw error;
      });
      entries.set(key, pending);
      if (entries.size > limit) entries.delete(entries.keys().next().value!);
      return pending;
    },
  };
}
