package com.asmolabs.vellum.repository;

import com.asmolabs.vellum.domain.entity.BookFormat;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.List;
import java.util.Optional;

@Repository
public interface BookFormatRepository extends JpaRepository<BookFormat, String> {
    List<BookFormat> findByBookId(String bookId);
    Optional<BookFormat> findByBookIdAndFormatIgnoreCase(String bookId, String format);
}
