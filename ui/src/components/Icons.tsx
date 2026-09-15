/**
 * Lantern icon set: inline SVG components.
 *
 * All icons use a 16×16 viewBox. Pass `className` for size and colour;
 * defaults to `w-4 h-4` and `currentColor`.
 */

interface IconProps {
  className?: string;
}

// ---------------------------------------------------------------------------
// Folder
// ---------------------------------------------------------------------------

export function FolderIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      {/* Tab */}
      <path d="M1 4.5C1 3.67 1.67 3 2.5 3H6l1.5 1.5H13.5c.83 0 1.5.67 1.5 1.5V12c0 .83-.67 1.5-1.5 1.5h-11C1.67 13.5 1 12.83 1 12V4.5z" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Link / bookmark URL
// ---------------------------------------------------------------------------

export function LinkIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      {/* Left link oval */}
      <path d="M6.5 9.5a3.18 3.18 0 01-4.5 0 3.18 3.18 0 010-4.5l1-1a3.18 3.18 0 014.5 0" />
      {/* Right link oval */}
      <path d="M9.5 6.5a3.18 3.18 0 014.5 0 3.18 3.18 0 010 4.5l-1 1a3.18 3.18 0 01-4.5 0" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Separator
// ---------------------------------------------------------------------------

export function SeparatorIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <circle cx="4"  cy="8" r="1.2" />
      <circle cx="8"  cy="8" r="1.2" />
      <circle cx="12" cy="8" r="1.2" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Chevron right (tree expand indicator / breadcrumb separator)
// ---------------------------------------------------------------------------

export function ChevronRightIcon({ className = "w-3 h-3" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M6 4l4 4-4 4" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Pencil (rename / edit)
// ---------------------------------------------------------------------------

export function PencilIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M11 2.5l2.5 2.5L5 13.5H2.5V11L11 2.5z" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Trash (delete)
// ---------------------------------------------------------------------------

export function TrashIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M2.5 4.5h11M6 4.5V3h4v1.5M5.5 4.5v8a1 1 0 001 1h3a1 1 0 001-1v-8" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Grip (drag handle)
// ---------------------------------------------------------------------------

export function GripIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      {/* Two columns of three dots: a classic drag grip */}
      <circle cx="5.5" cy="4.5" r="1.1" />
      <circle cx="5.5" cy="8"   r="1.1" />
      <circle cx="5.5" cy="11.5" r="1.1" />
      <circle cx="9.5" cy="4.5" r="1.1" />
      <circle cx="9.5" cy="8"   r="1.1" />
      <circle cx="9.5" cy="11.5" r="1.1" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// X / Close (clear search, cancel, tab close)
// ---------------------------------------------------------------------------

export function XIcon({ className = "w-3 h-3" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M3 3l10 10M13 3L3 13" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Check (confirm action)
// ---------------------------------------------------------------------------

export function CheckIcon({ className = "w-3 h-3" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.2"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M2.5 8.5l4 4 7-8" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Search (magnifying glass)
// ---------------------------------------------------------------------------

export function SearchIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <circle cx="6.5" cy="6.5" r="4" />
      <path d="M9.5 9.5l3.5 3.5" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// ChevronDown (filter drawer toggle)
// ---------------------------------------------------------------------------

export function ChevronDownIcon({ className = "w-3 h-3" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2020/svg"
      className={className}
      aria-hidden
    >
      <path d="M4 6l4 4 4-4" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Gear (settings)
// ---------------------------------------------------------------------------

export function GearIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M6.5 1h3l.4 1.5a4.8 4.8 0 011.2.7l1.5-.5 1.5 2.6-1.2 1a4.9 4.9 0 010 1.4l1.2 1L12.6 11l-1.5-.5a4.8 4.8 0 01-1.2.7L9.5 15h-3l-.4-1.5a4.8 4.8 0 01-1.2-.7l-1.5.5L1.9 10.7l1.2-1a4.9 4.9 0 010-1.4l-1.2-1L3.4 4.7l1.5.5a4.8 4.8 0 011.2-.7L6.5 1zM8 10a2 2 0 100-4 2 2 0 000 4z"
      />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Plus (add / new)
// ---------------------------------------------------------------------------

export function PlusIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M8 2v12M2 8h12" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Sliders (rule-set / settings manage)
// ---------------------------------------------------------------------------

export function SlidersIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M2 4h12M2 8h12M2 12h12" />
      <circle cx="5"  cy="4"  r="1.5" fill="currentColor" stroke="none" />
      <circle cx="11" cy="8"  r="1.5" fill="currentColor" stroke="none" />
      <circle cx="6"  cy="12" r="1.5" fill="currentColor" stroke="none" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Copy (duplicate)
// ---------------------------------------------------------------------------

export function CopyIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <rect x="5.5" y="1.5" width="9" height="9" rx="1.5" />
      <path d="M10.5 10.5v2.5a1.5 1.5 0 01-1.5 1.5H2a1.5 1.5 0 01-1.5-1.5V6A1.5 1.5 0 012 4.5h2.5" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Lock (built-in / immutable)
// ---------------------------------------------------------------------------

export function LockIcon({ className = "w-3 h-3" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <rect x="3" y="7.5" width="10" height="7" rx="1.5" />
      <path d="M5 7.5V5a3 3 0 016 0v2.5" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Download / export (subtree export action): arrow into a tray
// ---------------------------------------------------------------------------

export function DownloadIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M8 2.5v8" />
      <path d="M5 7.5l3 3 3-3" />
      <path d="M2.5 13.5h11" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Home (breadcrumb root)
// ---------------------------------------------------------------------------

export function HomeIcon({ className = "w-3.5 h-3.5" }: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <path d="M2 7.5L8 2l6 5.5" />
      <path d="M3.5 6.5V13h3.25v-3h2.5v3H12.5V6.5" />
    </svg>
  );
}
