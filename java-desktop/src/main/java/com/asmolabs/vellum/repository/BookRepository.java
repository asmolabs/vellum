package com.asmolabs.vellum.repository;

import com.asmolabs.vellum.domain.entity.Book;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Query;
import org.springframework.data.repository.query.Param;
import org.springframework.stereotype.Repository;

import java.util.List;

@Repository
public interface BookRepository extends JpaRepository<Book, String> {

    @Query("""
        SELECT DISTINCT b FROM Book b 
        LEFT JOIN b.tags t 
        WHERE (:search IS NULL OR LOWER(b.title) LIKE LOWER(CONCAT('%', :search, '%')) 
               OR LOWER(b.author) LIKE LOWER(CONCAT('%', :search, '%')) 
               OR LOWER(b.description) LIKE LOWER(CONCAT('%', :search, '%')))
          AND (:tagName IS NULL OR t.name = :tagName)
        ORDER BY b.addedAt DESC
    """)
    List<Book> searchBooks(@Param("search") String search, @Param("tagName") String tagName);

    @Query("""
        SELECT DISTINCT b FROM Book b 
        LEFT JOIN b.tags t 
        WHERE (:search IS NULL OR LOWER(b.title) LIKE LOWER(CONCAT('%', :search, '%')) 
               OR LOWER(b.author) LIKE LOWER(CONCAT('%', :search, '%')) 
               OR LOWER(b.description) LIKE LOWER(CONCAT('%', :search, '%')))
          AND (:tagName IS NULL OR t.name = :tagName)
        ORDER BY b.addedAt DESC
    """)
    Page<Book> searchBooksPaged(@Param("search") String search, @Param("tagName") String tagName, Pageable pageable);
}
