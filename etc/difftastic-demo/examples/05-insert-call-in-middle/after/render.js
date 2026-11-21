function render(items) {
  const rows = items.map(toRow);
  return format(unique(sort(rows)));
}
