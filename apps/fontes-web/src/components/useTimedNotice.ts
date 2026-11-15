import { useEffect, useState } from "react";
/** Success feedback expires; repeating the same message restarts its timer. */
export function useTimedNotice(duration = 3500) {
  const [value, setValue] = useState<{ text: string } | null>(null);
  useEffect(() => {
    if (!value) return;
    const timer = window.setTimeout(() => setValue(null), duration);
    return () => window.clearTimeout(timer);
  }, [value, duration]);
  return [value?.text || "", (text: string) => setValue(text ? { text } : null)] as const;
}
