# Context menus: token, position, transaction, link, image, selection and page menus.

# Source: scripts/ui/context_menu/builders.js

## Token and position items

menu-token-buy = Buy Token
menu-token-sell = Sell Token
menu-token-blacklist = Blacklist Token
menu-token-refresh = Refresh Data
menu-view-details = View Details
menu-favorite-add = Add to Favorites
menu-favorite-remove = Remove from Favorites
menu-copy-address = Copy Address
menu-copy-symbol = Copy Symbol
menu-position-sell = Sell { $symbol }
menu-position-add = Add to Position
menu-position-management = Management

## Transaction, link, image, selection and page items

menu-copy-signature = Copy Signature
menu-link-open = Open Link
menu-link-open-new-tab = Open in New Tab
menu-copy-link-address = Copy Link Address
menu-copy-link-text = Copy Link Text
menu-image-open = Open Image
menu-copy-image-address = Copy Image Address
menu-search-google = Search on Google
menu-page-reload = Reload

## Names of copied values, shown in the copy confirmation

menu-copied-token-address = Token address
menu-copied-symbol = Symbol
menu-copied-transaction-signature = Transaction signature
menu-copied-link = Link
menu-copied-link-text = Link text
menu-copied-image-url = Image URL
menu-copied-text = Text

# Source: scripts/ui/context_menu.js
menu-inspect-element = Inspect Element

# Source: scripts/ui/context_menu/actions.js

## Favorites

menu-token-fallback = Token
menu-favorite-added = { $symbol } added to favorites
menu-favorite-removed = { $symbol } removed from favorites
menu-favorite-add-failed = Failed to add favorite
menu-favorite-remove-failed = Failed to remove favorite
menu-favorite-update-failed = Failed to update favorites

## Blacklist and refresh

menu-blacklist-confirm-message = Are you sure you want to blacklist { $symbol }? This token will be excluded from trading.
menu-blacklist-confirm-action = Blacklist
menu-blacklist-done = { $symbol } blacklisted
menu-blacklist-request-failed = Failed to blacklist token
menu-blacklist-failed = Failed to blacklist
menu-refresh-done = Token data refreshed
menu-refresh-request-failed = Failed to refresh token data
menu-refresh-failed = Failed to refresh
