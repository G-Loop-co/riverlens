import { useTranslation } from "react-i18next";
import { cloneElement, useId, type ReactElement, type ReactNode } from "react";
import { WarningCircle, Tray } from "@phosphor-icons/react";
export function ErrorBanner({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  return (
    <div role="alert" className="error-banner">
      <WarningCircle size={20} />
      <div>{typeof children === "string" ? t(children) : children}</div>
    </div>
  );
}
export function Empty({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <div className="empty-state">
      <div className="empty-icon">
        <Tray size={32} weight="duotone" />
      </div>
      <h2>{t(title)}</h2>
      <p>{t(description)}</p>
      {action}
    </div>
  );
}
export function Skeleton({ rows = 4 }: { rows?: number }) {
  const { t } = useTranslation();
  return (
    <div aria-label={t("載入中")} className="skeleton">
      {Array.from({ length: rows }, (_, i) => (
        <div
          className="skeleton-line"
          key={i}
          style={{ width: i === 0 ? "40%" : "100%", height: i === 0 ? 20 : 45 }}
        />
      ))}
    </div>
  );
}
export function Signed({
  value,
  suffix = "",
  digits = 2,
}: {
  value: number;
  suffix?: string;
  digits?: number;
}) {
  return (
    <span
      className={`mono ${value > 0 ? "positive" : value < 0 ? "negative" : ""}`}
    >
      {value > 0 ? "+" : ""}
      {value.toFixed(digits)}
      {suffix}
    </span>
  );
}
export function Field({
  label,
  children,
  help,
}: {
  label: string;
  children: ReactElement<{ id?: string; "aria-describedby"?: string }>;
  help?: string;
}) {
  const { t } = useTranslation();
  const generatedId = useId();
  const id = children.props.id || generatedId;
  const description = [children.props["aria-describedby"], help && `${id}-help`]
    .filter(Boolean)
    .join(" ");
  return (
    <div className="field">
      <label htmlFor={id}>{t(label)}</label>
      {cloneElement(children, {
        id,
        "aria-describedby": description || undefined,
      })}
      {help && <small id={`${id}-help`}>{t(help)}</small>}
    </div>
  );
}
