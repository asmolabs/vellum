package com.asmolabs.vellum.repository;

import com.asmolabs.vellum.domain.entity.Tag;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Query;
import org.springframework.stereotype.Repository;

import java.util.List;

@Repository
public interface TagRepository extends JpaRepository<Tag, String> {

    interface TagCountProjection {
        String getName();
        Long getBookCount();
    }

    @Query("""
        SELECT t.name AS name, COUNT(b.id) AS bookCount 
        FROM Tag t 
        JOIN Book b ON t MEMBER OF b.tags 
        GROUP BY t.name 
        ORDER BY t.name ASC
    """)
    List<TagCountProjection> findTagsWithBookCounts();
}
