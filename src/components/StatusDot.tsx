export function StatusDot({ active = true }: { active?: boolean }) {
  return <span className={active ? "status-dot" : "status-dot status-dot--idle"} aria-hidden="true" />;
}
