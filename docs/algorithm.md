Triangle Tower Slicing Algorithm 
    Given set of triangles T and Vertices V

    Triangles should be made CCW

    For each vertex in V 
        Create a new tower vertex in TV

    For each triangle t in T
        For each edge e with points (s -> f) in t
        if s < e
            Add the fragment [t,e] to Tv[s]
        else
            Add the fragment [e,t] to TV[f]

    for each tv in TV
        Combine the fragments in tv (join_fragments)
        Add tv to priority queue P

    height set to 0
    current_rings set to empty array
    While P is not empty
        current_vertex = P.pop
        Break apart current rings at any instance of current vertex ( split_on_edge ) and store in fragments
        Add any fragment from current vertex to fragments
        Join fragments (join_fragments) in fragments and store into current rings
        assert that each ring is complete ( first element matches last element)
        if P.peek.height > height + layer_height
            Output points into layers
            height += layer_height

split_on_edge algorithm

    parameter (ring , split_edge)

        new_ring = []
        finished_fragments = []

        for e in ring.elements 
            if e is edge
                if e is split_edge
                    Add new_ring to finished_fragments 
                    new_ring = []
                else
                    add e to new_ring 

            else //e is face
                add e to new_ring 

        Append new_ring to the start of finished_fragment[0] if it exists otherwise add new_ring to finished_fragments  

        Remove any fragments from finished_fragments that are 1 length faces //these are faces that ended there

        return finished_fragments

    examples
        Example 1
            input
                edge e2
                ring [e1,f1,e2,f2,e1]
            output
                ring[[f2,e1,f1]]
        Example 2
            input
                edge e2
                ring [e1,f1,e2,f4,e2,e1]
            output
                ring[[e1,f1]]

join_fragments algorithm
    //Optomizations exist like sorting the fragments

    parameter (fragments)

        for each fragment pair ( f1, f2)
            if last element of f1 matches first element of f2
                append f2 to f1 removing the duplicate matching element

    examples
        Example 1
            input
                fragments [[e1,f1,e2],[e2,f2,e1]]
            output
                ring[[e1,f1,e2,f2,e1]]
        Example 2
            input
                fragments [[e1,f1],[f3,e4],[f1,e3],[e5,f3],[e4,f3,e5]]
            output
                ring[[e1,f1,e3],[f3,e4,f3,e5,f3]]