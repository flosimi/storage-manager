type UsageBarProps = {
  percent: number;
};

function usageLevel(percent: number): string {
  if (percent > 90) return "critical";
  if (percent > 70) return "warn";
  return "ok";
}

export function UsageBar({ percent }: UsageBarProps) {
  return (
    <div className="bar">
      <div
        className={`bar-fill ${usageLevel(percent)}`}
        style={{ width: `${percent}%` }}
      />
    </div>
  );
}