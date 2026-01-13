#!/bin/bash
# Script to delete all branches except main
# This script must be run by a user with appropriate GitHub permissions

echo "This script will delete all branches except 'main'"
echo "Current branches to be deleted:"
echo ""

# List all branches except main
git branch -r | grep -v "main" | grep -v "HEAD" | sed 's/origin\///'

echo ""
echo "WARNING: This operation cannot be undone!"
echo "Press Ctrl+C to cancel, or Enter to continue..."
read -r

# Delete all remote branches except main
for branch in $(git branch -r | grep -v "main" | grep -v "HEAD" | sed 's/origin\///'); do
    echo "Deleting branch: $branch"
    git push origin --delete "$branch"
done

echo ""
echo "Branch cleanup complete!"
echo "Remaining branches:"
git branch -r
