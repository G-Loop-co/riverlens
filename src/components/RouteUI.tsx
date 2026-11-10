import {
  Component,
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useState,
} from "react";
import { WarningCircle } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { errorText } from "../api";

export interface RouteTabItem<T extends string> {
  value: T;
  label: string;
}

interface RouteTabsProps<T extends string> {
  label: string;
  panelId: string;
  items: readonly RouteTabItem<T>[];
  value: T;
  onChange: (value: T) => void;
}

export function RouteTabs<T extends string>({
  label,
  panelId,
  items,
  value,
  onChange,
}: RouteTabsProps<T>) {
  const { t } = useTranslation();

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    const index = items.findIndex((item) => item.value === value);
    if (index < 0) return;
    let next: number | null = null;
    if (event.key === "ArrowRight") next = (index + 1) % items.length;
    if (event.key === "ArrowLeft")
      next = (index - 1 + items.length) % items.length;
    if (event.key === "Home") next = 0;
    if (event.key === "End") next = items.length - 1;
    if (next === null) return;
    event.preventDefault();
    const target = event.currentTarget
      .closest("[role=tablist]")
      ?.querySelectorAll<HTMLButtonElement>("[role=tab]");
    onChange(items[next].value);
    target?.[next]?.focus();
  }

  return (
    <nav className="route-tabs" role="tablist" aria-label={t(label)}>
      {items.map((item) => (
        <button
          key={item.value}
          id={`${panelId}-tab-${item.value}`}
          role="tab"
          type="button"
          className={`route-tab ${item.value === value ? "active" : ""}`}
          aria-selected={item.value === value}
          aria-controls={panelId}
          tabIndex={item.value === value ? 0 : -1}
          onKeyDown={handleKeyDown}
          onClick={() => onChange(item.value)}
        >
          {t(item.label)}
        </button>
      ))}
    </nav>
  );
}

export function TabPanel({
  id,
  activeTab,
  children,
}: {
  id: string;
  activeTab: string;
  children: ReactNode;
}) {
  return (
    <div
      id={id}
      role="tabpanel"
      aria-labelledby={`${id}-tab-${activeTab}`}
      tabIndex={0}
    >
      {children}
    </div>
  );
}

export function WorkspaceHeading({
  eyebrow,
  title,
  description,
  action,
}: {
  eyebrow: string;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <div className="page-heading">
      <div>
        <div className="eyebrow">{eyebrow}</div>
        <h1>{t(title)}</h1>
        <p>{t(description)}</p>
      </div>
      {action}
    </div>
  );
}

export function StatusPill({
  tone = "online",
  children,
}: {
  tone?: "online" | "offline" | "busy";
  children: ReactNode;
}) {
  return (
    <span className={`status-pill ${tone}`} aria-live="polite">
      <i aria-hidden="true" />
      {children}
    </span>
  );
}

function RouteErrorView({
  error,
  onRetry,
}: {
  error: string;
  onRetry: () => void;
}) {
  const { t } = useTranslation();
  return (
    <section className="route-error" role="alert">
      <WarningCircle size={24} />
      <h2>{t("此工作區暫時無法顯示")}</h2>
      <p>{error}</p>
      <button className="button" onClick={onRetry}>
        {t("重試")}
      </button>
    </section>
  );
}

interface BoundaryState {
  error: string | null;
  revision: number;
}

export class RouteErrorBoundary extends Component<
  { children: ReactNode },
  BoundaryState
> {
  state: BoundaryState = { error: null, revision: 0 };

  static getDerivedStateFromError(error: unknown): BoundaryState {
    return { error: errorText(error), revision: 0 };
  }

  componentDidCatch(error: unknown) {
    console.error("RiverLens workspace error", error);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <RouteErrorView
        error={this.state.error}
        onRetry={() =>
          this.setState((state) => ({
            error: null,
            revision: state.revision + 1,
          }))
        }
      />
    );
  }
}

export function RouteMount({
  routeKey,
  children,
}: {
  routeKey: string;
  children: ReactNode;
}) {
  const [revision, setRevision] = useState(0);
  useEffect(() => setRevision(0), [routeKey]);
  return <RouteErrorBoundary key={revision}>{children}</RouteErrorBoundary>;
}
