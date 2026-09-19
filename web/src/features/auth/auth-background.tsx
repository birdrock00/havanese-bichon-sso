// Decorative background for the login branding panel: a single, very subtle
// fine line grid. The soft glows in the panel provide the color depth.
// Theme tokens are used, so it adapts to light/dark automatically.

export function AuthBackground() {
  return (
    <div aria-hidden='true' className='pointer-events-none absolute inset-0 overflow-hidden'>
      {/* Thin line grid */}
      <div
        className='absolute inset-0 opacity-15'
        style={{
          backgroundImage:
            'linear-gradient(hsl(var(--border)) 1px, transparent 1px), linear-gradient(90deg, hsl(var(--border)) 1px, transparent 1px)',
          backgroundSize: '56px 56px',
        }}
      />
    </div>
  )
}
