function render(items) {
  const rows = items.map(toRow);
  return format(sort(rows));
}
