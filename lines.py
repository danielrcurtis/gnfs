import os
from pathlib import Path

def count_lines_of_code(base_dir):
    total_lines = 0
    rs_files_count = 0
    # Walk through all directories and files in the base directory
    for root, dirs, files in os.walk(base_dir):
        for file in files:
            file_path = Path(root) / file
            # Check if the file is a Rust source file
            if file_path.suffix == '.rs':
                rs_files_count += 1
                with open(file_path, 'r', encoding='utf-8') as f:
                    # Increment total lines by the number of lines in this file
                    total_lines += sum(1 for line in f)
    
    return total_lines, rs_files_count

# Example usage
base_directory = './'
lines_of_code, files_counted = count_lines_of_code(base_directory)
print(f"Total .rs files counted: {files_counted}")
print(f"Total lines of Rust code: {lines_of_code}")
