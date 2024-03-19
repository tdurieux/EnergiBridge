import nbformat


def extract_and_write_code(notebook_path, start_marker, end_marker):
    extracted_code = []
    is_extracting = False
    
    with open(notebook_path, 'r', encoding='utf-8') as notebook_file:
        notebook_content = nbformat.read(notebook_file, as_version=4)
        
        for cell in notebook_content.cells:
            if cell.cell_type == 'code':
                source_code = cell.source
                if start_marker in source_code:
                    is_extracting = True
                    # Exclude the start marker 
                    source_code = source_code.split(start_marker, 1)[-1]
                if is_extracting:
                    extracted_code.append(source_code)
                if end_marker in source_code:
                    is_extracting = False
                    # Exclude the end marker
                    extracted_code[-1] = extracted_code[-1].rsplit(end_marker, 1)[0]
    
    # Write extracted code to a new Python file(temp.py)
    with open('temp.py', 'w', encoding='utf-8') as temp_file:
        temp_file.write('\n'.join(extracted_code))


#params
notebook_path = 'Demo.ipynb'
start_marker = '#EnergiBridgeStart'
end_marker = '#EnergiBridgeStop'


